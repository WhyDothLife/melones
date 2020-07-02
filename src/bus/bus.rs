use super::super::cpu;

pub struct bus{
    cpu: cpu::cpu::CPU_6502,
    cpu_ram: [u8; 2048],
    system_clock_counter: u32
}

impl bus{
    pub fn cpu_write(&mut self, addr: u16, data: u8){
        if addr >= 0x0000 && addr <= 0x1FFF{
            self.cpu_ram[addr & 0x07FF as usize] = data;
        }
    }

    pub fn cpu_read(&self, addr: u16) -> u8{
        if addr >= 0x0000 && addr <= 0x1FFF{
            return self.cpu_ram[addr & 0x07FF as usize];
        }
    }
}