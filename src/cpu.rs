#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::convert::TryInto;

use crate::bus;

// enum INSTRUCTION{
//     name,
//     operate,
//     addrmode,
// }

// static lookup: vec!(INSTRUCTION) = {

// }

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
    clock_count: u32 // accumulation of the number of clocks
}

// pub enum FLAGS_6502{
//     C = (1 << 0),	// Carry Bit
// 	Z = (1 << 1),	// Zero
// 	I = (1 << 2),	// Disable Interrupts
// 	D = (1 << 3),	// Decimal Mode (unused by NES)
// 	B = (1 << 4),	// Break
// 	U = (1 << 5),	// Unused
// 	V = (1 << 6),	// Overflow
// 	N = (1 << 7),	// Negative
// }

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
    pub fn new() -> Self{
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
            clock_count: 0
        };
        return cpu; 
    }

    // Read and write a byte to a specific memory address
    fn read_this(&self, bus: &mut bus::Bus, a: u16) -> u8{
        return bus.cpu_read(a);
    }
    fn write_this(&self, bus: &mut bus::Bus, a: u16, d: u8){
        bus.cpu_write(a, d);
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

    pub fn reset(&mut self, bus: &mut bus::Bus){
        // Get address to set program counter
        self.addr_abs = 0xFFFC;
        let lo: u16 = self.read_this(bus, self.addr_abs + 0).into();
        let hi: u16 = self.read_this(bus, self.addr_abs + 1).into();

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
    pub fn irq(&mut self, bus: &mut bus::Bus){
        if self.get_flag('I') == 0{
            // Push program counter to the stack
            self.write_this(bus, 0x0100 + self.stkp as u16, ((self.pc >> 8) & 0x00FF).try_into().unwrap());
            self.stkp -= 1;
            self.write_this(bus, 0x0100 + self.stkp as u16, (self.pc & 0x00FF).try_into().unwrap());
            self.stkp -= 1;

            // Push status register to the stack
            self.set_flag('B', false);
            self.set_flag('U', true);
            self.set_flag('I', true);
            self.write_this(bus, 0x0100 + self.stkp as u16, self.status);
            self.stkp -= 1;

            // Read new program counter location
            self.addr_abs = 0xFFFE;
            let lo: u16 = self.read_this(bus, self.addr_abs + 0).into();
            let hi: u16 = self.read_this(bus, self.addr_abs + 1).into();
            self.pc = (hi << 8) | lo;

            self.cycles = 7;
        }
    }

    // Non-Maskable Interrupt - cannot be ignored
    pub fn nmi(&mut self, bus: &mut bus::Bus){
        self.write_this(bus, 0x0100 + self.stkp as u16, ((self.pc >> 8) & 0x00FF).try_into().unwrap());
        self.stkp -= 1;
        self.write_this(bus, 0x0100 + self.stkp as u16, (self.pc & 0x00FF).try_into().unwrap());
        self.stkp -= 1;

        self.set_flag('B', false);
        self.set_flag('U', true);
        self.set_flag('I', true);
        self.write_this(bus, 0x0100 + self.stkp as u16, self.status);
        self.stkp -= 1;

        self.addr_abs = 0xFFFA;
        let lo: u16 = self.read_this(bus, self.addr_abs + 0).into();
        let hi: u16 = self.read_this(bus, self.addr_abs + 1).into();
        self.pc = (hi << 8) | lo;

        self.cycles = 8;
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
     fn ZP0(&mut self, bus: &mut bus::Bus) -> u8{
        self.addr_abs = self.read_this(bus, self.pc).into();
        self.pc += 1;
        self.addr_abs &= 0x00FF;
        return 0;
     }
     // Zero page with X offset
     fn ZPX(&mut self, bus: &mut bus::Bus) -> u8{
        self.addr_abs = (self.read_this(bus, self.pc) + self.x).into();
        self.pc += 1;
        self.addr_abs &= 0x00FF;
        return 0;
     }
     // Zero page with Y offset
     fn ZPY(&mut self, bus: &mut bus::Bus) -> u8{
        self.addr_abs = (self.read_this(bus, self.pc) + self.y).into();
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
     // Absolute with X Offset
     fn ABX(&mut self, bus: &mut bus::Bus) -> u8{
        let lo: u16 = self.read_this(bus, self.pc) .into();
        self.pc += 1;
        let hi: u16 = self.read_this(bus, self.pc) .into();
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
     fn ABY(&mut self, bus: &mut bus::Bus) -> u8{
        let lo: u16 = self.read_this(bus, self.pc) .into();
        self.pc += 1;
        let hi: u16 = self.read_this(bus, self.pc) .into();
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
     fn IND(&mut self, bus: &mut bus::Bus) -> u8{
        let ptr_lo: u16 = self.read_this(bus, self.pc) .into();
        self.pc += 1;
        let ptr_hi: u16 = self.read_this(bus, self.pc) .into();
        self.pc += 1;

        let ptr: u16 = (ptr_hi << 8) | ptr_lo;

        // Bug in NES
        if ptr_lo == 0x00FF{ // Should be fine 
            self.addr_abs = ((self.read_this(bus, ptr & 0xFF00) as u16) >> 8) | (self.read_this(bus, ptr + 0) as u16);
        }else{               // Should be fine
            self.addr_abs = ((self.read_this(bus, ptr + 1) as u16) << 8) | (self.read_this(bus, ptr + 0) as u16);
        }

        return 0;
     }
     // Indirect X
     fn IZX(&mut self, bus: &mut bus::Bus) -> u8{
        let t: u16 = self.read_this(bus, self.pc).into();
        self.pc += 1;

        let lo: u16 = self.read_this(bus, (t + self.x as u16) & 0x00FF).into();
        let hi: u16 = self.read_this(bus, (t + self.x as u16 + 1) & 0x00FF).into();

        self.addr_abs = (hi << 8) | lo;

        return 0;
     }
     // Indirect Y
     fn IZY(&mut self, bus: &mut bus::Bus) -> u8{
        let t: u16 = self.read_this(bus, self.pc).into();
        self.pc += 1;

        let lo: u16 = self.read_this(bus, t & 0x00FF).into();
        let hi: u16 = self.read_this(bus, (t + 1) & 0x00FF).into();

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
    fn fetch(&mut self, _bus: &mut bus::Bus) -> u8{
        // ADD LATER
        // if addrmode_lookup[self.opcode] == &IMP{
        //     self.fetched = self.read_this(bus, self.addr_abs);
        // }
        return self.fetched;
    }

    /**********************************
     * 
     * Instruction Implementations
     * 
     **********************************/
    // Add with Carry In
     fn ADC(&mut self, bus: &mut bus::Bus) -> u8{
        // Grab data for accumulator
        self.fetch(bus);

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
    fn SBC(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);

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
    fn AND(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.accum &= self.fetched;
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', self.accum & 0x80 != 0);
        return 1;
    }
    // Arithmetic Shift Left
    fn ASL(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.temp = (self.fetched << 1) as u16;
        self.set_flag('C', (self.temp & 0xFF00) > 0);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x00);
        self.set_flag('N', (self.temp & 0x80) != 0);
        // ADD LATER
        // if addrmode_lookup[self.opcode] == &IMP{
        //     self.fetched = self.read_this(bus, self.addr_abs);
        // }else{
        //     self.write_this(bus, self.addr_abs, self.temp & 0x00FF);
        // }
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
    fn BIT(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
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
    fn BRK(&mut self, bus: &mut bus::Bus) -> u8{
        self.pc += 1;

        self.set_flag('I', true);
        self.write_this(bus, 0x0100 + self.stkp as u16, ((self.pc >> 8) & 0x00FF) as u8);
        self.stkp -= 1;
        self.write_this(bus, 0x0100 + self.stkp as u16, (self.pc & 0x00FF) as u8);
        self.stkp -= 1;

        self.set_flag('B', true);
        self.write_this(bus, 0x0100 + self.stkp as u16, self.status);
        self.stkp -= 1;
        self.set_flag('B', false);

        self.pc = (self.read_this(bus, 0xFFFE) | ((self.read_this(bus, 0xFFFF) as u16) << 8) as u8) as u16;
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
    fn CMP(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.temp = (self.accum - self.fetched) as u16;
        self.set_flag('C', self.accum >= self.fetched);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        return 1;
    }
    // Compare X Register
    fn CPX(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.temp = (self.x - self.fetched) as u16;
        self.set_flag('C', self.x >= self.fetched);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        return 0;
    }
    // Compare Y Register
    fn CPY(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.temp = (self.y - self.fetched) as u16;
        self.set_flag('C', self.y >= self.fetched);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        return 1;
    }
    // Decrement Value at Memory Location
    fn DEC(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.temp = (self.fetched - 1) as u16;
        self.write_this(bus, self.addr_abs, (self.temp & 0x00FF) as u8);
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
    fn EOR(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.accum = self.accum ^ self.fetched;
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', (self.accum & 0x80) != 0);
        return 1;
    }
    // Increment Value at Memory Location
    fn INC(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.temp = (self.fetched + 1) as u16;
        self.write_this(bus, self.addr_abs, (self.temp & 0x00FF) as u8);
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
    fn JSR(&mut self, bus: &mut bus::Bus) -> u8{
        self.pc -= 1;

        self.write_this(bus, 0x0100 + self.stkp as u16, ((self.pc >> 8) & 0x00FF) as u8);
        self.stkp -= 1;
        self.write_this(bus, 0x0100 + self.stkp as u16, (self.pc & 0x00FF) as u8);
        self.stkp -= 1;

        self.pc = self.addr_abs;
        return 0;
    }
    // Load The Accumulator
    fn LDA(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.accum = self.fetched;
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', (self.accum & 0x80) != 0);
        return 1;
    }
    // Load The X Register
    fn LDX(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.x = self.fetched;
        self.set_flag('Z', self.x == 0x00);
        self.set_flag('N', (self.x & 0x80) != 0);
        return 1;
    }
    // Load The Y Register
    fn LDY(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.y = self.fetched;
        self.set_flag('Z', self.y == 0x00);
        self.set_flag('N', (self.y & 0x80) != 0);
        return 1;
    }
    
    fn LSR(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.set_flag('C', (self.fetched & 0x0001) != 0);
        self.temp = (self.fetched >> 1) as u16;
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);

        // if addrmode_lookup[self.opcode] == &IMP{
        //     self.accum = self.temp & 0x00FF;
        // }else{
        //     self.write_this(bus, self.addr_abs, (self.temp & 0x00FF) as u8);
        // }
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
    fn ORA(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.accum = self.accum | self.fetched;
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', (self.accum & 0x80) != 0);
        return 1;
    }
    // Push Accumulator to Stack
    fn PHA(&mut self, bus: &mut bus::Bus) -> u8{
        self.write_this(bus, 0x0100 + self.stkp as u16, self.accum);
        self.stkp -= 1;
        return 0;
    }
    // Push Status Register to Stack
    fn PHP(&mut self, bus: &mut bus::Bus) -> u8{
        self.write_this(bus, 0x0100 + self.stkp as u16, self.status | FLAGS_6502('B') | FLAGS_6502('U'));
        self.set_flag('B', false);
        self.set_flag('U', false);
        self.stkp -= 1;
        return 0;
    }
    // Pop Accumulator off Stack
    fn PLA(&mut self, bus: &mut bus::Bus) -> u8{
        self.stkp += 1;
        self.accum = self.read_this(bus, 0x0100 + self.stkp as u16);
        self.set_flag('Z', self.accum == 0x00);
        self.set_flag('N', (self.accum & 0x80) != 0);
        return 0;
    }
    // Pop Status Register off Stack
    fn PLP(&mut self, bus: &mut bus::Bus) -> u8{
        self.stkp += 1;
        self.status = self.read_this(bus, 0x0100 + self.stkp as u16);
        self.set_flag('U', true);
        return 0;
    }

    fn ROL(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.temp = ((self.fetched << 1) | self.get_flag('C')) as u16;
        self.set_flag('C', (self.temp & 0xFF00) != 0);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x0000);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        
        // if addrmode_lookup[self.opcode] == &IMP{
        //     self.accum = (self.temp & 0x00FF) as u8;
        // }else{
        //     self.write_this(bus, self.addr_abs, (self.temp & 0x00FF) as u8);
        // }
        return 0;
    }

    fn ROR(&mut self, bus: &mut bus::Bus) -> u8{
        self.fetch(bus);
        self.temp = ((self.get_flag('C') << 7) | (self.fetched >> 1)) as u16;
        self.set_flag('C', (self.fetched & 0x01) != 0);
        self.set_flag('Z', (self.temp & 0x00FF) == 0x00);
        self.set_flag('N', (self.temp & 0x0080) != 0);
        
        // if addrmode_lookup[self.opcode] == &IMP{
        //     self.accum = (self.temp & 0x00FF) as u8;
        // }else{
        //     self.write_this(bus, self.addr_abs, (self.temp & 0x00FF) as u8);
        // }
        return 0;
    }

    fn RTI(&mut self, bus: &mut bus::Bus) -> u8{
        self.stkp += 1;
        self.status = self.read_this(bus, 0x0100 + self.stkp as u16);
        self.status &= !FLAGS_6502('B');
        self.status &= !FLAGS_6502('U');

        self.stkp += 1;
        self.pc = self.read_this(bus, 0x0100 + self.stkp as u16) as u16;
        self.stkp += 1;
        self.pc |= self.read_this(bus, (0x0100 + self.stkp as u16) << 8) as u16;
        return 0;
    }

    fn RTS(&mut self, bus: &mut bus::Bus) -> u8{
        self.stkp += 1;
        self.pc = self.read_this(bus, 0x0100 + self.stkp as u16) as u16;
        self.stkp += 1;
        self.pc |= self.read_this(bus, (0x0100 + self.stkp as u16) << 8) as u16;

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
    fn STA(&mut self, bus: &mut bus::Bus) -> u8{
        self.write_this(bus, self.addr_abs, self.accum);
        return 0;
    }
    // Store X Register at Address
    fn STX(&mut self, bus: &mut bus::Bus) -> u8{
        self.write_this(bus, self.addr_abs, self.x);
        return 0;
    }
    // Store Y Register at Address
    fn STY(&mut self, bus: &mut bus::Bus) -> u8{
        self.write_this(bus, self.addr_abs, self.y);
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
}