

struct GBVideo {
    display_enable: bool,
    window_offset: u16,
    window_enable: bool,
    bg_offset: u16,
    bg_enable: bool,
    tile_data_select: bool,
    sprite_size: bool,
    sprite_enable: bool,

    scroll_y: u8,
    scroll_x: u8,
    lcdc_y: u8,
    ly_compare: u8,
    window_y: u8,
    window_x: u8,
    bg_palette: u8,
    obj_palette_0: u8,
    obj_palette_1: u8,
    
    tile_mem: Vec<u8>,
    sprite_mem: Vec<u8>,
}

impl MemDevice for GBVideo {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0x8000...0x9FFF => self.tile_mem[(loc - 0x8000) as usize],
            0xFE00...0xFE9F => self.sprite_mem[(loc - 0xFE00) as usize],
            0xFF40 => self.lcd_control_read(),
            0xFF41 => self.lcd_status_read(),
            0xFF42 => self.scroll_y,
            0xFF43 => self.scroll_x,
            0xFF44 => self.lcdc_y,
            0xFF45 => self.ly_compare,
            0xFF47 => self.bg_palette,
            0xFF48 => self.obj_palette_0,
            0xFF49 => self.obj_palette_1,
            0xFF4A => self.window_y,
            0xFF4B => self.window_x,
            _ => 0,
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            0x8000...0x9FFF => self.tile_mem[(loc - 0x8000) as usize] = val,
            0xFE00...0xFE9F => self.sprite_mem[(loc - 0xFE00) as usize] = val,
            0xFF40 => self.lcd_control_write(val),
            0xFF41 => self.lcd_status_write(val),
            0xFF42 => self.scroll_y = val,
            0xFF43 => self.scroll_x = val,
            0xFF44 => self.lcdc_y = val,
            0xFF45 => self.ly_compare = val,
            0xFF47 => self.bg_palette = val,
            0xFF48 => self.obj_palette_0 = val,
            0xFF49 => self.obj_palette_1 = val,
            0xFF4A => self.window_y = val,
            0xFF4B => self.window_x = val,
            _ => return,
        }
    }
}


impl GBVideo {
    // Drawing for a single frame
    pub fn render_frame(&mut self) {
    }

    fn lcd_control_write(&mut self, val: u8) {
        self.display_enable = if (val & 0x80 == 0x80) {true} else {false};
        self.window_offset = if (val & 0x40 == 0x40) {0x9C00} else {0x9800};
        self.window_enable = if (val & 0x20 == 0x20) {true} else {false};
        self.tile_data_select = if (val & 0x10 == 0x10) {true} else {false};
        self.bg_offset = if (val & 0x8 == 0x8) {0x9C00} else {0x9800};
        self.sprite_size = if (val & 0x4 == 0x4) {true} else {false};
        self.sprite_enable = if (val & 0x2 == 0x2) {true} else {false};
        self.bg_enable = if (val & 0x1 == 0x1) {true} else {false};
    }

    fn lcd_control_read(&self) -> u8 {
        let val_7 = if self.display_enable {0x80} else {0};
        let val_6 = if (self.window_offset == 0x9C00) {0x40} else{0};
        let val_5 = if self.window_enable {0x20} else {0};
        let val_4 = if self.tile_data_select {0x10} else {0};
        let val_3 = if (self.bg_offset == 0x9C00) {0x8} else {0};
        let val_2 = if self.sprite_size {0x4} else {0};
        let val_1 = if self.sprite_enable {0x2} else {0};
        let val_0 = if self.bg_enable {0x1} else {0};
        val_7 | val_6 | val_5 | val_4 | val_3 | val_2 | val_1 | val_0;
    }

    fn lcd_status_write(&mut self, val: u8) {
    }

    fn lcd_status_read(&self) -> u8 {
        0
    }


}
