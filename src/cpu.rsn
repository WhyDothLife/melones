use std::convert::TryInto;

use crate::bus;

// This is copied from FCEU.
static CYCLE_TABLE: [u8; 256] = [
    /*0x00*/ 7, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6, /*0x10*/ 2, 5, 2, 8, 4, 4,
    6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /*0x20*/ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6,
    /*0x30*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /*0x40*/ 6, 6, 2, 8, 3, 3,
    5, 5, 3, 2, 2, 2, 3, 4, 6, 6, /*0x50*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
    /*0x60*/ 6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6, /*0x70*/ 2, 5, 2, 8, 4, 4,
    6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /*0x80*/ 2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,
    /*0x90*/ 2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5, /*0xA0*/ 2, 6, 2, 6, 3, 3,
    3, 3, 2, 2, 2, 2, 4, 4, 4, 4, /*0xB0*/ 2, 5, 2, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4,
    /*0xC0*/ 2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, /*0xD0*/ 2, 5, 2, 8, 4, 4,
    6, 6, 2, 4, 2, 7, 4, 4, 7, 7, /*0xE0*/ 2, 6, 3, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,
    /*0xF0*/ 2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
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
        C => (1 << 0),	// Carry Bit
	    Z => (1 << 1),	// Zero
	    I => (1 << 2),	// Disable Interrupts
	    D => (1 << 3),	// Decimal Mode (unused by NES)
	    B => (1 << 4),	// Break
	    U => (1 << 5),	// Unused
	    V => (1 << 6),	// Overflow
	    N => (1 << 7),	// Negative
    }
}

impl CPU_6502{
    pub fn new() -> Self{
        let mut cpu = CPU_6502{
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
    fn read_this(&self, bus: &bus::Bus, a: u16) -> u8{
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

    pub fn reset(&mut self, bus: &bus::Bus){
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
     fn imp(&mut self) -> u8{
        self.fetched = self.accum;
        return 0;
     }
     // Immediate
     fn imm(&mut self) -> u8{
        self.addr_abs = self.pc;
        self.pc += 1;
        return 0;
     }
     // Zero page
     fn zp0(&mut self, bus: &bus::Bus) -> u8{
        self.addr_abs = self.read_this(bus, self.pc).into();
        self.pc += 1;
        self.addr_abs &= 0x00FF;
        return 0;
     }
     // Zero page with X offset
     fn zpx(&mut self, bus: &bus::Bus) -> u8{
        self.addr_abs = (self.read_this(bus, self.pc) + self.x).into();
        self.pc += 1;
        self.addr_abs &= 0x00FF;
        return 0;
     }
     // Zero page with Y offset
     fn zpy(&mut self, bus: &bus::Bus) -> u8{
        self.addr_abs = (self.read_this(bus, self.pc) + self.y).into();
        self.pc += 1;
        self.addr_abs &= 0x00FF;
        return 0;
     }
     // Relative
     fn rel(&mut self, bus: &bus::Bus) -> u8{
        self.addr_abs = self.pc;
        self.pc += 1;
        if self.addr_rel & 0x80 == 1{
            self.addr_rel |= 0xFF00;
        }
        return 0;
     }
     // Absolute with X Offset
     fn abx(&mut self, bus: &bus::Bus) -> u8{
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
     fn aby(&mut self, bus: &bus::Bus) -> u8{
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
     fn ind(&mut self, bus: &bus::Bus) -> u8{
        let ptr_lo: u16 = self.read_this(bus, self.pc) .into();
        self.pc += 1;
        let ptr_hi: u16 = self.read_this(bus, self.pc) .into();
        self.pc += 1;

        let ptr: u16 = (ptr_hi << 8) | ptr_lo;

        
        if ptr_lo == 0x00FF{ // Should be fine 
            self.addr_abs = ((self.read_this(bus, ptr & 0xFF00) >> 8) | self.read_this(bus, ptr + 0)).into();
        }else{               // Should be fine
            self.addr_abs = ((self.read_this(bus, ptr + 1) << 8) | self.read_this(bus, ptr + 0)).into();
        }

        return 0;
     }
     // Indirect X
     fn izx(&mut self, bus: &bus::Bus) -> u8{
        let t: u16 = self.read_this(bus, self.pc).into();
        self.pc += 1;

        let lo: u16 = self.read_this(bus, (t + self.x as u16) & 0x00FF).into();
        let hi: u16 = self.read_this(bus, (t + self.x as u16 + 1) & 0x00FF).into();

        self.addr_abs = (hi << 8) | lo;

        return 0;
     }
     // Indirect Y
     fn izy(&mut self, bus: &bus::Bus) -> u8{
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
}