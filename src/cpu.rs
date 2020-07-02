use std::convert::TryInto;

use crate::bus;

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
    fn read_this(&self, bus: &bus::bus, a: u16) -> u8{
        return bus.cpu_read(a);
    }
    fn write_this(&self, bus: &mut bus::bus, a: u16, d: u8){
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

    pub fn reset(&mut self, bus: &bus::bus){
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
    pub fn irq(&mut self, bus: &mut bus::bus){
        if self.get_flag('I') == 0{
            // Push program counter to the stack
            self.write_this(bus, (0x0100 + self.stkp).into(), ((self.pc >> 8) & 0x00FF).try_into().unwrap());
            self.stkp -= 1;
            self.write_this(bus, (0x0100 + self.stkp).into(), (self.pc & 0x00FF).try_into().unwrap());
            self.stkp -= 1;

            // Push status register to the stack
            self.set_flag('B', false);
            self.set_flag('U', true);
            self.set_flag('I', true);
            self.write_this(bus, (0x0100 + self.stkp).into(), self.status);
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
    pub fn nmi(&mut self, bus: &mut bus::bus){
        self.write_this(bus, (0x0100 + self.stkp).into(), ((self.pc >> 8) & 0x00FF).try_into().unwrap());
        self.stkp -= 1;
        self.write_this(bus, (0x0100 + self.stkp).into(), (self.pc & 0x00FF).try_into().unwrap());
        self.stkp -= 1;

        self.set_flag('B', false);
        self.set_flag('U', true);
        self.set_flag('I', true);
        self.write_this(bus, (0x0100 + self.stkp).into(), self.status);
        self.stkp -= 1;

        self.addr_abs = 0xFFFA;
        let lo: u16 = self.read_this(bus, self.addr_abs + 0).into();
        let hi: u16 = self.read_this(bus, self.addr_abs + 1).into();
        self.pc = (hi << 8) | lo;

        self.cycles = 8;
    }
}