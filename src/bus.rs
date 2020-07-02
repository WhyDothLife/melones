pub struct Bus{
    cpu_ram: [u8; 2048],
    system_clock_counter: u32
}

impl Bus{
    pub fn new() -> Self{
        let mut b = Bus{
            cpu_ram: [0; 2048],
            system_clock_counter: 0
        };
        return b;
    }
    
    pub fn cpu_write(&mut self, addr: u16, data: u8){
        if addr >= 0x0000 && addr <= 0x1FFF{
            self.cpu_ram[(addr & 0x07FF) as usize] = data;
        }else{

        }
    }

    pub fn cpu_read(&self, addr: u16) -> u8{
        if addr >= 0x0000 && addr <= 0x1FFF{
            return self.cpu_ram[(addr & 0x07FF) as usize];
        }else{
            return 0;
        }
    }
}