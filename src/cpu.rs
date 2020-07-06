#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::convert::TryInto;

use crate::bus;

// This is copied from FCEU.
static CYCLE_TABLE: [u8; 256] = [
    /*0x00*/ 7, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6, 
    /*0x10*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, 
    /*0x20*/ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6,
    /*0x30*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, 
    /*0x40*/ 6, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6, 
    /*0x50*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x60*/ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6, 
    /*0x70*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, 
    /*0x80*/ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
    /*0x90*/ 2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5, 
    /*0xA0*/ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4, 
    /*0xB0*/ 2, 5, 2, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4,
    /*0xC0*/ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, 
    /*0xD0*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, 
    /*0xE0*/ 2, 6, 3, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    /*0xF0*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7
];

pub struct CPU_6502{
    accum: u8, // Accumulator register
    x: u8,     // X register
    y: u8,     // Y register
    stkp: u8,  // Stack pointer (points to location on bus)
    pc: u16,   // Program counter
    status: u8, // Status register
    fetched: u8,
    temp: u16,
    addr_abs: u16,
    addr_rel: u16,
    opcode: u8, // Instruction byte
    cycles: u8, // cycles remaining
    clock_count: u32, // accumulation of the number of clocks
    bus: bus::Bus
}

fn FLAGS_6502(c: char) -> u8{
    match c{
        'C' => return 1 << 0,	// Carry Bit
	    'Z' => return 1 << 1,	// Zero
	    'I' => return 1 << 2,	// Disable Interrupts
	    'D' => return 1 << 3,	// Decimal Mode (unused by NES)
	    'B' => return 1 << 4,	// Break
	    'U' => return 1 << 5,	// Unused
	    'V' => return 1 << 6,	// Overflow
        'N' => return 1 << 7,	// Negative
         _  => {println!("{}", 0); return 0;}
    }
}

impl CPU_6502{
    pub fn new(xBus: bus::Bus) -> Self{
        let cpu = CPU_6502{
            accum: 0,
            x: 0,
            y: 0,
            pc: 0,
            status: 0x24,
            stkp: 0xFD,
            fetched: 0x00,
            temp: 0x0000,
            addr_abs: 0x0000,
            addr_rel: 0x00,
            opcode: 0x00,
            cycles: 0,
            clock_count: 0,
            bus: xBus
        };
        return cpu; 
    }

    // Read and write a byte to a specific memory address
    fn read_this(&self, a: u16) -> u8{
        return self.bus.cpu_read(a);
    }
    fn write_this(&mut self, a: u16, d: u8){
        self.bus.cpu_write(a, d);
    }

    // Gets and sets flags for convienance
    fn get_flag(&self, f: char) -> u8{
        if (self.status & FLAGS_6502(f)) > 0{
            return 1;
        }else{
            return 0;
        }
    }
    fn set_flag(&mut self, f: char, z: bool){
        if z{
            self.status |= FLAGS_6502(f);
        }else{
            self.status &= !FLAGS_6502(f);
        }
    }

    pub fn reset(&mut self){
        // Get address to set program counter
        self.addr_abs = 0xFFFC;
        let lo: u16 = self.read_this(self.addr_abs + 0).into();
        let hi: u16 = self.read_this(self.addr_abs + 1).into();

        // Set counter
        self.pc = (hi << 8) | lo;

        // Reset registers
        self.accum = 0;
        self.x = 0;
        self.y = 0;
        self.stkp = 0xFD;
        self.status = 0x00 | FLAGS_6502('U');

        // Clear helpers
        self.addr_rel = 0x0000;
        self.addr_abs = 0x0000;
        self.fetched = 0x00;

        self.cycles = 8;
    }

    // Interrupt Request - can be ignored
    pub fn irq(&mut self){
        if self.get_flag('I') == 0{
            // Push program counter to the stack
            self.write_this(0x0100 + self.stkp as u16, ((self.pc >> 8) & 0x00FF).try_into().unwrap());
            self.stkp -= 1;
            self.write_this(0x0100 + self.stkp as u16, (self.pc & 0x00FF).try_into().unwrap());
            self.stkp -= 1;

            // Push status register to the stack
            self.set_flag('B', false);
            self.set_flag('U', true);
            self.set_flag('I', true);
            self.write_this(0x0100 + self.stkp as u16, self.status);
            self.stkp -= 1;

            // Read new program counter location
            self.addr_abs = 0xFFFE;
            let lo: u16 = self.read_this(self.addr_abs + 0).into();
            let hi: u16 = self.read_this(self.addr_abs + 1).into();
            self.pc = (hi << 8) | lo;

            self.cycles = 7;
        }
    }

    // Non-Maskable Interrupt - cannot be ignored
    pub fn nmi(&mut self){
        self.write_this(0x0100 + self.stkp as u16, ((self.pc >> 8) & 0x00FF).try_into().unwrap());
        self.stkp -= 1;
        self.write_this(0x0100 + self.stkp as u16, (self.pc & 0x00FF).try_into().unwrap());
        self.stkp -= 1;

        self.set_flag('B', false);
        self.set_flag('U', true);
        self.set_flag('I', true);
        self.write_this(0x0100 + self.stkp as u16, self.status);
        self.stkp -= 1;

        self.addr_abs = 0xFFFA;
        let lo: u16 = self.read_this(self.addr_abs + 0).into();
        let hi: u16 = self.read_this(self.addr_abs + 1).into();
        self.pc = (hi << 8) | lo;

        self.cycles = 8;
    }

    // One cycle of emulation
    fn clock(&mut self){
        if self.cycles == 0{
            self.opcode = self.read_this(self.pc);

            self.set_flag('U', true);

            self.pc += 1;

            self.cycles = INSTRUCTIONS[self.opcode as usize].cycles();

            let more_cycles1: u8 = match INSTRUCTIONS[self.opcode as usize].addr_mode(){
                IMM => self.IMM(),
                IMP => self.IMP(),
                ZP0 => self.ZP0(),
                ZPX => self.ZPX(),
                ZPY => self.ZPY(),
                ABS => self.ABS(),
                ABX => self.ABX(),
                ABY => self.ABY(),
                IND => self.IND(),
                IZX => self.IZX(),
                IZY => self.IZX(),
                REL => self.REL()
            };

            let more_cycles2: u8 = match INSTRUCTIONS[self.opcode as usize].oper(){
                ADC => self.ADC(),
                AND => self.AND(),
                ASL => self.ASL(),
                BCC => self.BCC(),
                BCS => self.BCS(),
                BEQ => self.BEQ(),
                BIT => self.BIT(),
                BMI => self.BMI(),
                BNE => self.BNE(),
                BPL => self.BPL(),
                BRK => self.BRK(),
                BVC => self.BVC(),
                BVS => self.BVS(),
                CLC => self.CLC(),
                CLD => self.CLD(),
                CLI => self.CLI(),
                CLV => self.CLV(),
                CMP => self.CMP(),
                CPX => self.CPX(),
                CPY => self.CPY(),
                DEC => self.DEC(),
                DEX => self.DEX(),
                DEY => self.DEY(),
                EOR => self.EOR(),
                INC => self.INC(),
                INX => self.INX(),
                INY => self.INY(),
                JMP => self.JMP(),
                JSR => self.JSR(),
                LDA => self.LDA(),
                LDX => self.LDX(),
                LDY => self.LDY(),
                LSR => self.LSR(),
                NOP => self.NOP(),
                ORA => self.ORA(),
                PHA => self.PHA(),
                PHP => self.PHP(),
                PLA => self.PLA(),
                PLP => self.PLP(),
                ROL => self.ROL(),
                ROR => self.ROR(),
                RTI => self.RTI(),
                RTS => self.RTS(),
                SBC => self.SBC(),
                SEC => self.SEC(),
                SED => self.SED(),
                SEI => self.SEI(),
                STA => self.STA(),
                STX => self.STX(),
                STY => self.STY(),
                TAX => self.TAX(),
                TAY => self.TAY(),
                TSX => self.TSX(),
                TXA => self.TXA(),
                TXS => self.TXS(),
                TYA => self.TYA(),
                _   => self.XXX(INSTRUCTIONS[self.opcode as usize].opcode())
            };

            self.cycles += more_cycles1 & more_cycles2;

            self.set_flag('U', true);
        }

        self.cycles -= 1;
    }

    /**********************************
     * 
     * Addressing Modes
     * 
     **********************************/
     // Implied
    fn IMP(&mut self) -> u8{
        self.fetched = self.accum;
        return 0;
     }
     // Immediate
    fn IMM(&mut self) -> u8{
        self.addr_abs = self.pc;
        self.pc += 1;
        return 0;
     }
     // Zero page
    fn ZP0(&mut self) -> u8{
        self.addr_abs = self.read_this(self.pc).into();
        self.pc += 1;
        self.addr_abs &= 0x00FF;
        return 0;
     }
     // Zero page with X offset
    fn ZPX(&mut self) -> u8{
        self.addr_abs = (self.read_this(self.pc) + self.x).into();
        self.pc += 1;
        self.addr_abs &= 0x00FF;
        return 0;
     }
     // Zero page with Y offset
    fn ZPY(&mut self) -> u8{
        self.addr_abs = (self.read_this(self.pc) + self.y).into();
        self.pc += 1;
        self.addr_abs &= 0x00FF;
        return 0;
     }
     // Relative
    fn REL(&mut self) -> u8{
        self.addr_abs = self.pc;
        self.pc += 1;
        if self.addr_rel & 0x80 == 1{
            self.addr_rel |= 0xFF00;
        }
        return 0;
     }
     // Absolute
     fn ABS(&mut self) -> u8{
        let lo: u16 = self.read_this(self.pc) as u16;
        self.pc += 1;
        let hi: u16 = self.read_this(self.pc) as u16;
        self.pc += 1;
        
        self.addr_abs = (hi << 8) | lo;

        return 0;
     }
     // Absolute with X Offset
    fn ABX(&mut self) -> u8{
        let lo: u16 = self.read_this(self.pc) .into();
        self.pc += 1;
        let hi: u16 = self.read_this(self.pc) .into();
        self.pc += 1;
        
        self.addr_abs = (hi << 8) | lo;
        self.addr_abs += self.x as u16;

        if (self.addr_abs & 0xFF00) != (hi << 8){
            return 1;
        }else{
            return 0;
        }
     }
     // Absolute with Y offset
    fn ABY(&mut self) -> u8{
        let lo: u16 = self.read_this(self.pc) .into();
        self.pc += 1;
        let hi: u16 = self.read_this(self.pc) .into();
        self.pc += 1;
        
        self.addr_abs = (hi << 8) | lo;
        self.addr_abs += self.y as u16;

        if (self.addr_abs & 0xFF00) != (hi << 8){
            return 1;
        }else{
            return 0;
        }
     }
     // Indirect
    fn IND(&mut self) -> u8{
        let ptr_lo: u16 = self.read_this(self.pc) .into();
        self.pc += 1;
        let ptr_hi: u16 = self.read_this(self.pc) .into();
        self.pc += 1;

        let ptr: u16 = (ptr_hi << 8) | ptr_lo;

        // Bug in NES
        if ptr_lo == 0x00FF{ // Should be fine 
            self.addr_abs = ((self.read_this(ptr & 0xFF00) as u16) >> 8) | (self.read_this(ptr + 0) as u16);
        }else{               // Should be fine
            self.addr_abs = ((self.read_this(ptr + 1) as u16) << 8) | (self.read_this(ptr + 0) as u16);
        }

        return 0;
     }
     // Indirect X
    fn IZX(&mut self) -> u8{
        let t: u16 = self.read_this(self.pc).into();
        self.pc += 1;

        let lo: u16 = self.read_this((t + self.x as u16) & 0x00FF).into();
        let hi: u16 = self.read_this((t + self.x as u16 + 1) & 0x00FF).into();

        self.addr_abs = (hi << 8) | lo;

        return 0;
    }
     // Indirect Y
    fn IZY(&mut self) -> u8{
        let t: u16 = self.read_this(self.pc).into();
        self.pc += 1;

        let lo: u16 = self.read_this(t & 0x00FF).into();
        let hi: u16 = self.read_this((t + 1) & 0x00FF).into();

        self.addr_abs = (hi << 8) | lo;
        self.addr_abs += self.y as u16;

        if (self.addr_abs & 0xFF00) != (hi << 8){
            return 1;
        }else{
            return 0;
        }
    }
    
     /**************
     DONT FORGET TO FINISH THIS
    *************************/
    // Fetches the data used by the instruction
    fn fetch(&mut self) -> u8{
        if INSTRUCTIONS[self.opcode as usize].addr_mode() != IMP{
            self.fetched = self.read_this(self.addr_abs);
        }
        return self.fetched;
    }

    /**********************************
     * 
     * Instruction Implementations
     * 
     **********************************/
    // Add with Carry In
     fn ADC(&mut self) -> u8{
        // Grab data for accumulator
        self.fetch();

        // Performed in 16 bit to capture a carry bit
        // This will exist in bit 8 of the 16 bit
        self.temp = (self.accum + self.fetched + self.get_flag('C')) as u16;

        self.set_flag('C', self.temp > 255);

        self.set_flag('Z', (self.temp & 0x00FF) == 0);

        self.set_flag('V', !((self.accum ^ self.fetched) as u16 & (self.accum as u16 ^ self.temp)) & 0x0080 != 0);

        self.set_flag('N', self.temp & 0x80 != 0);

        self.accum = (self.temp & 0x00FF) as u8;

        return 1;
    }
    // Subtraction with Borrow In
    fn SBC(&mut self) -> u8{
        self.fetch();

        let value: u16 = self.fetched as u16 ^ 0x00FF;

        self.temp = self.accum as u16 + value + self.get_flag('C') as u16;
        self.set_flag('C', (self.temp & 0xFF00) != 0);
        self.set_flag('Z', (self.temp & 0x00FF) == 0);
        self.set_flag('V', ((self.temp ^ self.accum as u16) & (self.temp ^ value) & 0x0080) != 0);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        self.accum = (self.temp & 0x00FF) as u8;
        return 1;
    }
    // Bitwise Logic AND
    fn AND(&mut self) -> u8{
        self.fetch();
        self.accum &= self.fetched;
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', self.accum & 0x80 != 0);
        return 1;
    }
    // Arithmetic Shift Left
    fn ASL(&mut self) -> u8{
        self.fetch();
        self.temp = (self.fetched << 1) as u16;
        self.set_flag('C', (self.temp & 0xFF00) > 0);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x00);
        self.set_flag('N', (self.temp & 0x80) != 0);

        if INSTRUCTIONS[self.opcode as usize].addr_mode() != IMP{
            self.fetched = self.read_this(self.addr_abs);
        }else{
            self.write_this(self.addr_abs, (self.temp & 0x00FF) as u8);
        }
        return 0;
    }
    // Branch if Carry Clear
    fn BCC(&mut self) -> u8{
        if self.get_flag('C') == 0{
            self.cycles += 1;
            self.addr_abs = self.pc + self.addr_rel;

            if (self.addr_abs & 0xFF00) != (self.pc & 0xFF00){
                self.cycles += 1;
            }

            self.pc = self.addr_abs;
        }
        return 0;
    }
    // Branch if Carry Set
    fn BCS(&mut self) -> u8{
        if self.get_flag('C') == 1{
            self.cycles += 1;
            self.addr_abs = self.pc + self.addr_rel;

            if (self.addr_abs & 0xFF00) != (self.pc & 0xFF00){
                self.cycles += 1;
            }

            self.pc = self.addr_abs;
        }
        return 0;
    }
    // Branch if equal
    fn BEQ(&mut self) -> u8{
        if self.get_flag('Z') == 1{
            self.cycles += 1;
            self.addr_abs = self.pc + self.addr_rel;

            if (self.addr_abs & 0xFF00) != (self.pc & 0xFF00){
                self.cycles += 1;
            }

            self.pc = self.addr_abs;
        }
        return 0;
    }
    // 
    fn BIT(&mut self) -> u8{
        self.fetch();
        self.temp = (self.accum & self.fetched) as u16;
        self.set_flag('Z', (self.temp & 0x00FF) == 0x00);
        self.set_flag('N', (self.fetched & (1 << 7)) != 0);
        self.set_flag('V', (self.fetched & (1 << 6)) != 0);
        return 0;
    }
    // Branch if Negative
    fn BMI(&mut self) -> u8{
        if self.get_flag('N') == 1{
            self.cycles += 1;
            self.addr_abs = self.pc + self.addr_rel;

            if (self.addr_abs & 0xFF00) != (self.pc & 0xFF00){
                self.cycles += 1;
            }

            self.pc = self.addr_abs;
        }
        return 0;
    }
    // Branch if Not Equal
    fn BNE(&mut self) -> u8{
        if self.get_flag('Z') == 0{
            self.cycles += 1;
            self.addr_abs = self.pc + self.addr_rel;

            if (self.addr_abs & 0xFF00) != (self.pc & 0xFF00){
                self.cycles += 1;
            }

            self.pc = self.addr_abs;
        }
        return 0;
    }
    // Branch if Positive
    fn BPL(&mut self) -> u8{
        if self.get_flag('N') == 1{
            self.cycles += 1;
            self.addr_abs = self.pc + self.addr_rel;

            if (self.addr_abs & 0xFF00) != (self.pc & 0xFF00){
                self.cycles += 1;
            }

            self.pc = self.addr_abs;
        }
        return 0;
    }
    // Break
    fn BRK(&mut self) -> u8{
        self.pc += 1;

        self.set_flag('I', true);
        self.write_this(0x0100 + self.stkp as u16, ((self.pc >> 8) & 0x00FF) as u8);
        self.stkp -= 1;
        self.write_this(0x0100 + self.stkp as u16, (self.pc & 0x00FF) as u8);
        self.stkp -= 1;

        self.set_flag('B', true);
        self.write_this(0x0100 + self.stkp as u16, self.status);
        self.stkp -= 1;
        self.set_flag('B', false);

        self.pc = (self.read_this(0xFFFE) | ((self.read_this(0xFFFF) as u16) << 8) as u8) as u16;
        return 0;
    }
    // Branch if Overflow Clear
    fn BVC(&mut self) -> u8{
        if self.get_flag('V') == 0{
            self.cycles += 1;
            self.addr_abs = self.pc + self.addr_rel;

            if (self.addr_abs & 0xFF00) != (self.pc & 0xFF00){
                self.cycles += 1;
            }

            self.pc = self.addr_abs;
        }
        return 0;
    }
    // Branch if Overflow Set
    fn BVS(&mut self) -> u8{
        if self.get_flag('V') == 1{
            self.cycles += 1;
            self.addr_abs = self.pc + self.addr_rel;

            if (self.addr_abs & 0xFF00) != (self.pc & 0xFF00){
                self.cycles += 1;
            }

            self.pc = self.addr_abs;
        }
        return 0;
    }
    // Clear Carry Flag
    fn CLC(&mut self) -> u8{
        self.set_flag('C', false);
        return 0;
    }
    // Clear Decimal Flag
    fn CLD(&mut self) -> u8{
        self.set_flag('D', false);
        return 0;
    }
    // Disable Interrupts / Clear Interrupt Flag
    fn CLI(&mut self) -> u8{
        self.set_flag('I', false);
        return 0;
    }
    // Clear Overflow Flag
    fn CLV(&mut self) -> u8{
        self.set_flag('V', false);
        return 0;
    }
    // Compare Accumulator
    fn CMP(&mut self) -> u8{
        self.fetch();
        self.temp = (self.accum - self.fetched) as u16;
        self.set_flag('C', self.accum >= self.fetched);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        return 1;
    }
    // Compare X Register
    fn CPX(&mut self) -> u8{
        self.fetch();
        self.temp = (self.x - self.fetched) as u16;
        self.set_flag('C', self.x >= self.fetched);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        return 0;
    }
    // Compare Y Register
    fn CPY(&mut self) -> u8{
        self.fetch();
        self.temp = (self.y - self.fetched) as u16;
        self.set_flag('C', self.y >= self.fetched);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        return 1;
    }
    // Decrement Value at Memory Location
    fn DEC(&mut self) -> u8{
        self.fetch();
        self.temp = (self.fetched - 1) as u16;
        self.write_this(self.addr_abs, (self.temp & 0x00FF) as u8);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        return 1;
    }
    // Decrement X Register
    fn DEX(&mut self) -> u8{
        self.x -= 1;
        self.set_flag('Z', self.x == 0x00);
        self.set_flag('N', (self.x & 0x80) != 0);
        return 0;
    }
    // Decrement Y Register
    fn DEY(&mut self) -> u8{
        self.y -= 1;
        self.set_flag('Z', self.y == 0x00);
        self.set_flag('N', (self.y & 0x80) != 0);
        return 0;
    }
    // Bitwise Logic XOR
    fn EOR(&mut self) -> u8{
        self.fetch();
        self.accum = self.accum ^ self.fetched;
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', (self.accum & 0x80) != 0);
        return 1;
    }
    // Increment Value at Memory Location
    fn INC(&mut self) -> u8{
        self.fetch();
        self.temp = (self.fetched + 1) as u16;
        self.write_this(self.addr_abs, (self.temp & 0x00FF) as u8);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp& 0x0080) != 0);
        return 0;
    }
    // Increment X Register
    fn INX(&mut self) -> u8{
        self.x += 1;
        self.set_flag('Z', self.x == 0x00);
        self.set_flag('N', (self.x & 0x80) != 0);
        return 0;
    }
    // Increment Y Register
    fn INY(&mut self) -> u8{
        self.y += 1;
        self.set_flag('Z', self.y == 0x00);
        self.set_flag('N', (self.y & 0x80) != 0);
        return 0;
    }
    // Jump To Location
    fn JMP(&mut self) -> u8{
        self.pc = self.addr_abs;
        return 0;
    }
    // Jump To Location
    fn JSR(&mut self) -> u8{
        self.pc -= 1;

        self.write_this(0x0100 + self.stkp as u16, ((self.pc >> 8) & 0x00FF) as u8);
        self.stkp -= 1;
        self.write_this(0x0100 + self.stkp as u16, (self.pc & 0x00FF) as u8);
        self.stkp -= 1;

        self.pc = self.addr_abs;
        return 0;
    }
    // Load The Accumulator
    fn LDA(&mut self) -> u8{
        self.fetch();
        self.accum = self.fetched;
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', (self.accum & 0x80) != 0);
        return 1;
    }
    // Load The X Register
    fn LDX(&mut self) -> u8{
        self.fetch();
        self.x = self.fetched;
        self.set_flag('Z', self.x == 0x00);
        self.set_flag('N', (self.x & 0x80) != 0);
        return 1;
    }
    // Load The Y Register
    fn LDY(&mut self) -> u8{
        self.fetch();
        self.y = self.fetched;
        self.set_flag('Z', self.y == 0x00);
        self.set_flag('N', (self.y & 0x80) != 0);
        return 1;
    }
    
    fn LSR(&mut self) -> u8{
        self.fetch();
        self.set_flag('C', (self.fetched & 0x0001) != 0);
        self.temp = (self.fetched >> 1) as u16;
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);

        if INSTRUCTIONS[self.opcode as usize].addr_mode() != IMP{
            self.accum = (self.temp & 0x00FF) as u8;
        }else{
            self.write_this(self.addr_abs, (self.temp & 0x00FF) as u8);
        }
        return 0;
    }

    fn NOP(&mut self) -> u8{
        match self.opcode{
            0x1C|
            0x3C|
            0x5C|
            0x7C|
            0xDC|
            0xFC => {return 1;}
            _ => {return 0;}
        }
    }
    // Bitwise Logic OR
    fn ORA(&mut self) -> u8{
        self.fetch();
        self.accum = self.accum | self.fetched;
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', (self.accum & 0x80) != 0);
        return 1;
    }
    // Push Accumulator to Stack
    fn PHA(&mut self) -> u8{
        self.write_this(0x0100 + self.stkp as u16, self.accum);
        self.stkp -= 1;
        return 0;
    }
    // Push Status Register to Stack
    fn PHP(&mut self) -> u8{
        self.write_this(0x0100 + self.stkp as u16, self.status | FLAGS_6502('B') | FLAGS_6502('U'));
        self.set_flag('B', false);
        self.set_flag('U', false);
        self.stkp -= 1;
        return 0;
    }
    // Pop Accumulator off Stack
    fn PLA(&mut self) -> u8{
        self.stkp += 1;
        self.accum = self.read_this(0x0100 + self.stkp as u16);
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', (self.accum & 0x80) != 0);
        return 0;
    }
    // Pop Status Register off Stack
    fn PLP(&mut self) -> u8{
        self.stkp += 1;
        self.status = self.read_this(0x0100 + self.stkp as u16);
        self.set_flag('U', true);
        return 0;
    }

    fn ROL(&mut self) -> u8{
        self.fetch();
        self.temp = ((self.fetched << 1) | self.get_flag('C')) as u16;
        self.set_flag('C', (self.temp & 0xFF00) != 0);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        
        if INSTRUCTIONS[self.opcode as usize].addr_mode() != IMP{
            self.accum = (self.temp & 0x00FF) as u8;
        }else{
            self.write_this(self.addr_abs, (self.temp & 0x00FF) as u8);
        }
        return 0;
    }

    fn ROR(&mut self) -> u8{
        self.fetch();
        self.temp = ((self.get_flag('C') << 7) | (self.fetched >> 1)) as u16;
        self.set_flag('C', (self.fetched & 0x01) != 0);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x00);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        
        if INSTRUCTIONS[self.opcode as usize].addr_mode() != IMP{
            self.accum = (self.temp & 0x00FF) as u8;
        }else{
            self.write_this(self.addr_abs, (self.temp & 0x00FF) as u8);
        }
        return 0;
    }

    fn RTI(&mut self) -> u8{
        self.stkp += 1;
        self.status = self.read_this(0x0100 + self.stkp as u16);
        self.status &= !FLAGS_6502('B');
        self.status &= !FLAGS_6502('U');

        self.stkp += 1;
        self.pc = self.read_this(0x0100 + self.stkp as u16) as u16;
        self.stkp += 1;
        self.pc |= self.read_this((0x0100 + self.stkp as u16) << 8) as u16;
        return 0;
    }

    fn RTS(&mut self) -> u8{
        self.stkp += 1;
        self.pc = self.read_this(0x0100 + self.stkp as u16) as u16;
        self.stkp += 1;
        self.pc |= self.read_this((0x0100 + self.stkp as u16) << 8) as u16;

        self.pc += 1;
        return 0;
    }
    // Set Carry Flag
    fn SEC(&mut self) -> u8{
        self.set_flag('C', true);
        return 0;
    }
    // Set Decimal Flag
    fn SED(&mut self) -> u8{
        self.set_flag('D', true);
        return 0;
    }
    // Set Interrupt Flag
    fn SEI(&mut self) -> u8{
        self.set_flag('I', true);
        return 0;
    }
    // Store Accumulator at Address
    fn STA(&mut self) -> u8{
        self.write_this(self.addr_abs, self.accum);
        return 0;
    }
    // Store X Register at Address
    fn STX(&mut self) -> u8{
        self.write_this(self.addr_abs, self.x);
        return 0;
    }
    // Store Y Register at Address
    fn STY(&mut self) -> u8{
        self.write_this(self.addr_abs, self.y);
        return 0;
    }
    // Transfer Accumulator to X Register
    fn TAX(&mut self) -> u8{
        self.x = self.accum;
        self.set_flag('Z', self.x == 0x00);
        self.set_flag('N', (self.x & 0x80) != 0);
        return 0;
    }
    // Transfer Accumulator to Y Register
    fn TAY(&mut self) -> u8{
        self.y = self.accum;
        self.set_flag('Z', self.y == 0x00);
        self.set_flag('N', (self.y & 0x80) != 0);
        return 0;
    }
    // Transfer Stack Pointer to X Register
    fn TSX(&mut self) -> u8{
        self.x = self.stkp;
        self.set_flag('Z', self.x == 0x00);
        self.set_flag('N', (self.x & 0x80) != 0);
        return 0;
    }
    // Transfer X Register to Accumulator
    fn TXA(&mut self) -> u8{
        self.accum = self.x;
        self.set_flag('Z', self.x == 0x00);
        self.set_flag('N', (self.x & 0x80) != 0);
        return 0;
    }
    // Transfer X Register to Stack Pointer
    fn TXS(&mut self) -> u8{
        self.stkp = self.x;
        return 0;
    }
    // Transfer X Register to Accumulator
    fn TYA(&mut self) -> u8{
        self.accum = self.y;
        self.set_flag('Z', self.x == 0x00);
        self.set_flag('N', (self.x & 0x80) != 0);
        return 0;
    }

    /**********************************
     * 
     * Illegal Opcodes
     * 
     **********************************/
    fn XXX(&mut self, op: u8) -> u8{
        println!("Illegal Opcode: {}", op);
        return 0;
    }
}

// Sets up opcodes and cycles in a 16x16 array
// Will clean up later
#[derive(Copy, Clone)]
enum Operation{
    ADC, AND, ASL, BCC, BCS, BEQ, BIT, BMI, BNE, BPL, BRK, BVC, BVS, CLC, CLD, CLI, CLV, CMP, CPX,
    CPY, DEC, DEX, DEY, EOR, INC, INX, INY, JMP, JSR, LDA, LDX, LDY, LSR, NOP, ORA, PHA, PHP, PLA,
    PLP, ROL, ROR, RTI, RTS, SBC, SEC, SED, SEI, STA, STX, STY, TAX, TAY, TSX, TXA, TXS, TYA,
    // "Illegal" instructions
    AHX, ALR, ANC, ARR, AXS, DCP, ISC, KIL, LAS, LAX, RLA, RRA, SAX, SHX, SHY, SLO, SRE, TAS, XAA,
    // Catches invalid instructions
    XXX
}
#[derive(Copy, Clone, PartialEq)]
enum AddrMode{
    IMM, ZP0, 
    ZPX, ZPY,
    ABS, ABX, 
    ABY, IND, 
    IZX, IZY,
    REL, IMP
}
use Operation::*;
use AddrMode::*;
struct Instruction(u8, AddrMode, Operation, u8);
impl Instruction {
    pub fn opcode(&self) -> u8 {
        self.0
    }
    pub fn addr_mode(&self) -> AddrMode {
        self.1
    }
    pub fn oper(&self) -> Operation {
        self.2
    }
    pub fn cycles(&self) -> u8 {
        self.3
    }
}
const INSTRUCTIONS: [Instruction; 256] = [
    Instruction(0x00, IMM, BRK, 7), Instruction(0x01, IZX, ORA, 6), Instruction(0x02, IMP, XXX, 2), Instruction(0x03, IZX, SLO, 8), Instruction(0x04, ZP0, NOP, 3), Instruction(0x05, ZP0, ORA, 3), Instruction(0x06, ZP0, ASL, 5), Instruction(0x07, ZP0, SLO, 5), Instruction(0x08, IMP, PHP, 3), Instruction(0x09, IMM, ORA, 2), Instruction(0x0A, IMP, ASL, 2), Instruction(0x0B, IMM, ANC, 2), Instruction(0x0C, ABS, NOP, 4), Instruction(0x0D, ABS, ORA, 4), Instruction(0x0E, ABS, ASL, 6), Instruction(0x0F, ABS, SLO, 6),
    Instruction(0x10, REL, BPL, 2), Instruction(0x11, IZY, ORA, 5), Instruction(0x12, IMP, XXX, 2), Instruction(0x13, IZY, SLO, 8), Instruction(0x14, ZPX, NOP, 4), Instruction(0x15, ZPX, ORA, 4), Instruction(0x16, ZPX, ASL, 6), Instruction(0x17, ZPX, SLO, 6), Instruction(0x18, IMP, CLC, 2), Instruction(0x19, ABY, ORA, 4), Instruction(0x1A, IMP, NOP, 2), Instruction(0x1B, ABY, SLO, 7), Instruction(0x1C, ABX, NOP, 4), Instruction(0x1D, ABX, ORA, 4), Instruction(0x1E, ABX, ASL, 7), Instruction(0x1F, ABX, SLO, 7),
    Instruction(0x20, ABS, JSR, 6), Instruction(0x21, IZX, AND, 6), Instruction(0x22, IMP, XXX, 2), Instruction(0x23, IZX, RLA, 8), Instruction(0x24, ZP0, BIT, 3), Instruction(0x25, ZP0, AND, 3), Instruction(0x26, ZP0, ROL, 5), Instruction(0x27, ZP0, RLA, 5), Instruction(0x28, IMP, PLP, 4), Instruction(0x29, IMM, AND, 2), Instruction(0x2A, IMP, ROL, 2), Instruction(0x2B, IMM, ANC, 2), Instruction(0x2C, ABS, BIT, 4), Instruction(0x2D, ABS, AND, 4), Instruction(0x2E, ABS, ROL, 6), Instruction(0x2F, ABS, RLA, 6),
    Instruction(0x30, REL, BMI, 2), Instruction(0x31, IZY, AND, 5), Instruction(0x32, IMP, XXX, 2), Instruction(0x33, IZY, RLA, 8), Instruction(0x34, ZPX, NOP, 4), Instruction(0x35, ZPX, AND, 4), Instruction(0x36, ZPX, ROL, 6), Instruction(0x37, ZPX, RLA, 6), Instruction(0x38, IMP, SEC, 2), Instruction(0x39, ABY, AND, 4), Instruction(0x3A, IMP, NOP, 2), Instruction(0x3B, ABY, RLA, 7), Instruction(0x3C, ABX, NOP, 4), Instruction(0x3D, ABX, AND, 4), Instruction(0x3E, ABX, ROL, 7), Instruction(0x3F, ABX, RLA, 7),
    Instruction(0x40, IMP, RTI, 6), Instruction(0x41, IZX, EOR, 6), Instruction(0x42, IMP, XXX, 2), Instruction(0x43, IZX, SRE, 8), Instruction(0x44, ZP0, NOP, 3), Instruction(0x45, ZP0, EOR, 3), Instruction(0x46, ZP0, LSR, 5), Instruction(0x47, ZP0, SRE, 5), Instruction(0x48, IMP, PHA, 3), Instruction(0x49, IMM, EOR, 2), Instruction(0x4A, IMP, LSR, 2), Instruction(0x4B, IMM, ALR, 2), Instruction(0x4C, ABS, JMP, 3), Instruction(0x4D, ABS, EOR, 4), Instruction(0x4E, ABS, LSR, 6), Instruction(0x4F, ABS, SRE, 6),
    Instruction(0x50, REL, BVC, 2), Instruction(0x51, IZY, EOR, 5), Instruction(0x52, IMP, XXX, 2), Instruction(0x53, IZY, SRE, 8), Instruction(0x54, ZPX, NOP, 4), Instruction(0x55, ZPX, EOR, 4), Instruction(0x56, ZPX, LSR, 6), Instruction(0x57, ZPX, SRE, 6), Instruction(0x58, IMP, CLI, 2), Instruction(0x59, ABY, EOR, 4), Instruction(0x5A, IMP, NOP, 2), Instruction(0x5B, ABY, SRE, 7), Instruction(0x5C, ABX, NOP, 4), Instruction(0x5D, ABX, EOR, 4), Instruction(0x5E, ABX, LSR, 7), Instruction(0x5F, ABX, SRE, 7),
    Instruction(0x60, IMP, RTS, 6), Instruction(0x61, IZX, ADC, 6), Instruction(0x62, IMP, XXX, 2), Instruction(0x63, IZX, RRA, 8), Instruction(0x64, ZP0, NOP, 3), Instruction(0x65, ZP0, ADC, 3), Instruction(0x66, ZP0, ROR, 5), Instruction(0x67, ZP0, RRA, 5), Instruction(0x68, IMP, PLA, 4), Instruction(0x69, IMM, ADC, 2), Instruction(0x6A, IMP, ROR, 2), Instruction(0x6B, IMM, ARR, 2), Instruction(0x6C, IND, JMP, 5), Instruction(0x6D, ABS, ADC, 4), Instruction(0x6E, ABS, ROR, 6), Instruction(0x6F, ABS, RRA, 6),
    Instruction(0x70, REL, BVS, 2), Instruction(0x71, IZY, ADC, 5), Instruction(0x72, IMP, XXX, 2), Instruction(0x73, IZY, RRA, 8), Instruction(0x74, ZPX, NOP, 4), Instruction(0x75, ZPX, ADC, 4), Instruction(0x76, ZPX, ROR, 6), Instruction(0x77, ZPX, RRA, 6), Instruction(0x78, IMP, SEI, 2), Instruction(0x79, ABY, ADC, 4), Instruction(0x7A, IMP, NOP, 2), Instruction(0x7B, ABY, RRA, 7), Instruction(0x7C, ABX, NOP, 4), Instruction(0x7D, ABX, ADC, 4), Instruction(0x7E, ABX, ROR, 7), Instruction(0x7F, ABX, RRA, 7),
    Instruction(0x80, IMM, NOP, 2), Instruction(0x81, IZX, STA, 6), Instruction(0x82, IMM, NOP, 2), Instruction(0x83, IZX, SAX, 6), Instruction(0x84, ZP0, STY, 3), Instruction(0x85, ZP0, STA, 3), Instruction(0x86, ZP0, STX, 3), Instruction(0x87, ZP0, SAX, 3), Instruction(0x88, IMP, DEY, 2), Instruction(0x89, IMM, NOP, 2), Instruction(0x8A, IMP, TXA, 2), Instruction(0x8B, IMM, XAA, 2), Instruction(0x8C, ABS, STY, 4), Instruction(0x8D, ABS, STA, 4), Instruction(0x8E, ABS, STX, 4), Instruction(0x8F, ABS, SAX, 4),
    Instruction(0x90, REL, BCC, 2), Instruction(0x91, IZY, STA, 6), Instruction(0x92, IMP, XXX, 2), Instruction(0x93, IZY, AHX, 6), Instruction(0x94, ZPX, STY, 4), Instruction(0x95, ZPX, STA, 4), Instruction(0x96, ZPY, STX, 4), Instruction(0x97, ZPY, SAX, 4), Instruction(0x98, IMP, TYA, 2), Instruction(0x99, ABY, STA, 5), Instruction(0x9A, IMP, TXS, 2), Instruction(0x9B, ABY, TAS, 5), Instruction(0x9C, ABX, SHY, 5), Instruction(0x9D, ABX, STA, 5), Instruction(0x9E, ABY, SHX, 5), Instruction(0x9F, ABY, AHX, 5),
    Instruction(0xA0, IMM, LDY, 2), Instruction(0xA1, IZX, LDA, 6), Instruction(0xA2, IMM, LDX, 2), Instruction(0xA3, IZX, LAX, 6), Instruction(0xA4, ZP0, LDY, 3), Instruction(0xA5, ZP0, LDA, 3), Instruction(0xA6, ZP0, LDX, 3), Instruction(0xA7, ZP0, LAX, 3), Instruction(0xA8, IMP, TAY, 2), Instruction(0xA9, IMM, LDA, 2), Instruction(0xAA, IMP, TAX, 2), Instruction(0xAB, IMM, LAX, 2), Instruction(0xAC, ABS, LDY, 4), Instruction(0xAD, ABS, LDA, 4), Instruction(0xAE, ABS, LDX, 4), Instruction(0xAF, ABS, LAX, 4),
    Instruction(0xB0, REL, BCS, 2), Instruction(0xB1, IZY, LDA, 5), Instruction(0xB2, IMP, XXX, 2), Instruction(0xB3, IZY, LAX, 5), Instruction(0xB4, ZPX, LDY, 4), Instruction(0xB5, ZPX, LDA, 4), Instruction(0xB6, ZPY, LDX, 4), Instruction(0xB7, ZPY, LAX, 4), Instruction(0xB8, IMP, CLV, 2), Instruction(0xB9, ABY, LDA, 4), Instruction(0xBA, IMP, TSX, 2), Instruction(0xBB, ABY, LAS, 4), Instruction(0xBC, ABX, LDY, 4), Instruction(0xBD, ABX, LDA, 4), Instruction(0xBE, ABY, LDX, 4), Instruction(0xBF, ABY, LAX, 4),
    Instruction(0xC0, IMM, CPY, 2), Instruction(0xC1, IZX, CMP, 6), Instruction(0xC2, IMM, NOP, 2), Instruction(0xC3, IZX, DCP, 8), Instruction(0xC4, ZP0, CPY, 3), Instruction(0xC5, ZP0, CMP, 3), Instruction(0xC6, ZP0, DEC, 5), Instruction(0xC7, ZP0, DCP, 5), Instruction(0xC8, IMP, INY, 2), Instruction(0xC9, IMM, CMP, 2), Instruction(0xCA, IMP, DEX, 2), Instruction(0xCB, IMM, AXS, 2), Instruction(0xCC, ABS, CPY, 4), Instruction(0xCD, ABS, CMP, 4), Instruction(0xCE, ABS, DEC, 6), Instruction(0xCF, ABS, DCP, 6),
    Instruction(0xD0, REL, BNE, 2), Instruction(0xD1, IZY, CMP, 5), Instruction(0xD2, IMP, XXX, 2), Instruction(0xD3, IZY, DCP, 8), Instruction(0xD4, ZPX, NOP, 4), Instruction(0xD5, ZPX, CMP, 4), Instruction(0xD6, ZPX, DEC, 6), Instruction(0xD7, ZPX, DCP, 6), Instruction(0xD8, IMP, CLD, 2), Instruction(0xD9, ABY, CMP, 4), Instruction(0xDA, IMP, NOP, 2), Instruction(0xDB, ABY, DCP, 7), Instruction(0xDC, ABX, NOP, 4), Instruction(0xDD, ABX, CMP, 4), Instruction(0xDE, ABX, DEC, 7), Instruction(0xDF, ABX, DCP, 7),
    Instruction(0xE0, IMM, CPX, 2), Instruction(0xE1, IZX, SBC, 6), Instruction(0xE2, IMM, NOP, 2), Instruction(0xE3, IZX, ISC, 8), Instruction(0xE4, ZP0, CPX, 3), Instruction(0xE5, ZP0, SBC, 3), Instruction(0xE6, ZP0, INC, 5), Instruction(0xE7, ZP0, ISC, 5), Instruction(0xE8, IMP, INX, 2), Instruction(0xE9, IMM, SBC, 2), Instruction(0xEA, IMP, NOP, 2), Instruction(0xEB, IMM, SBC, 2), Instruction(0xEC, ABS, CPX, 4), Instruction(0xED, ABS, SBC, 4), Instruction(0xEE, ABS, INC, 6), Instruction(0xEF, ABS, ISC, 6),
    Instruction(0xF0, REL, BEQ, 2), Instruction(0xF1, IZY, SBC, 5), Instruction(0xF2, IMP, XXX, 2), Instruction(0xF3, IZY, ISC, 8), Instruction(0xF4, ZPX, NOP, 4), Instruction(0xF5, ZPX, SBC, 4), Instruction(0xF6, ZPX, INC, 6), Instruction(0xF7, ZPX, ISC, 6), Instruction(0xF8, IMP, SED, 2), Instruction(0xF9, ABY, SBC, 4), Instruction(0xFA, IMP, NOP, 2), Instruction(0xFB, ABY, ISC, 7), Instruction(0xFC, ABX, NOP, 4), Instruction(0xFD, ABX, SBC, 4), Instruction(0xFE, ABX, INC, 7), Instruction(0xFF, ABX, ISC, 7),
];