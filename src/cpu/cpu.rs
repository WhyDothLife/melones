use std::vec::Vec;
use super::super::bus;

pub struct CPU_6502{
    accum: u8, // Accumulator register
    x: u8,     // X register
    y: u8,     // Y register
    stkp: u8,  // Stack pointer (points to location on bus)
    pc: u16,   // Program counter
    status: u8, // Status register
    mem: Vec<u8>
}

pub enum FLAGS_6502{
    C = (1 << 0),	// Carry Bit
	Z = (1 << 1),	// Zero
	I = (1 << 2),	// Disable Interrupts
	D = (1 << 3),	// Decimal Mode (unused by NES)
	B = (1 << 4),	// Break
	U = (1 << 5),	// Unused
	V = (1 << 6),	// Overflow
	N = (1 << 7),	// Negative
}

impl CPU_6502{
    pub fn new() -> Self{
        let mut cpu = CPU_6502{
            mem: vec![0; 0x2000],
            accum: 0,
            x: 0,
            y: 0,
            pc: 0,
            status: 0x24,
            stkp: 0xFD
        };
        return cpu; 
    }

    // pub fn read(a: u16) -> u8{
    //     let bus_cpu = bus::bus;
    // }
}