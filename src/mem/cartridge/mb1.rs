enum BankingMode {
    ROM,
    RAM
}

pub struct MB1 {
    upper_select:   u8,
    lower_select:   u8,
    banking_mode:   BankingMode,
}

impl MB1 {
    pub fn new() -> MB1 {
        MB1 {
            upper_select:   0,
            lower_select:   0,
            banking_mode:   BankingMode::ROM,
        }
    }

    pub fn set_lower(&mut self, val: u8) {
        match val & 0x1F {
            0 => self.lower_select = 1,
            x => self.lower_select = x,
        }
    }

    pub fn set_upper(&mut self, val: u8) {
        self.upper_select = val & 0x03;
    }

    pub fn mem_type_select(&mut self, val: u8) {
        match val & 1 {
            1 => self.banking_mode = BankingMode::RAM,
            _ => self.banking_mode = BankingMode::ROM,
        }
    }

    pub fn get_rom_bank(&self) -> u8 {
        match self.banking_mode {
            BankingMode::ROM => (self.upper_select << 5) | self.lower_select,
            BankingMode::RAM => self.lower_select,
        }
    }

    pub fn get_ram_bank(&self) -> u8 {
        match self.banking_mode {
            BankingMode::ROM => 0,
            BankingMode::RAM => self.upper_select,
        }
    }
}
