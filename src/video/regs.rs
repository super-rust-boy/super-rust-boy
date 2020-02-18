use bitflags::bitflags;

use super::Mode;

bitflags! {
    #[derive(Default)]
    struct LCDControl: u8 {
        const ENABLE                    = bit!(7);
        const WINDOW_TILE_MAP_SELECT    = bit!(6);
        const WINDOW_DISPLAY_ENABLE     = bit!(5);
        const TILE_DATA_SELECT          = bit!(4);
        const BG_TILE_MAP_SELECT        = bit!(3);
        const OBJ_SIZE                  = bit!(2);
        const OBJ_DISPLAY_ENABLE        = bit!(1);
        const DISPLAY_PRIORITY          = bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    pub struct LCDStatusFlags: u8 {
        const COINCEDENCE_INT   = bit!(6);
        const OAM_INT           = bit!(5);
        const V_BLANK_INT       = bit!(4);
        const H_BLANK_INT       = bit!(3);
        const COINCEDENCE_FLAG  = bit!(2);
    }
}

#[derive(Clone)]
pub struct LCDStatus {
    flags: LCDStatusFlags,
    video_mode: Mode,
}

impl LCDStatus {
    fn new() -> Self {
        LCDStatus {
            flags: LCDStatusFlags::default(),
            video_mode: Mode::_2
        }
    }

    fn read(&self) -> u8 {
        self.flags.bits() | self.video_mode as u8
    }

    fn write(&mut self, val: u8) {
        self.flags = LCDStatusFlags::from_bits_truncate(val);
        self.video_mode = Mode::from(val);
    }

    fn read_flags(&self) -> LCDStatusFlags {
        self.flags
    }

    fn read_mode(&self) -> Mode {
        self.video_mode
    }

    fn write_mode(&mut self, mode: Mode) {
        self.video_mode = mode;
    }
}

// Video registers are copied across threads.
#[derive(Clone)]
pub struct VideoRegs {
    lcd_control:    LCDControl,
    lcd_status:     LCDStatus,
    lcdc_y:         u8,
    pub ly_compare: u8,

    pub scroll_y:   u8,
    pub scroll_x:   u8,
    pub window_y:   u8,
    pub window_x:   u8,
}

impl VideoRegs {
    pub fn new() -> Self {
        VideoRegs {
            lcd_control:    LCDControl::ENABLE,
            lcd_status:     LCDStatus::new(),
            lcdc_y:         0,
            ly_compare:     0,

            scroll_y:       0,
            scroll_x:       0,
            window_y:       0,
            window_x:       0,
        }
    }

    pub fn compare_ly_equal(&self) -> bool {
        self.lcdc_y == self.ly_compare
    }

    pub fn read_flags(&self) -> LCDStatusFlags {
        self.lcd_status.read_flags()
    }

    pub fn read_mode(&self) -> Mode {
        self.lcd_status.read_mode()
    }

    pub fn is_display_enabled(&self) -> bool {
        self.lcd_control.contains(LCDControl::ENABLE)
    }

    // For rendering background.
    pub fn get_background_priority(&self) -> bool {
        self.lcd_control.contains(LCDControl::DISPLAY_PRIORITY)
    }

    // For rendering window.
    pub fn get_window_enable(&self) -> bool {
        self.lcd_control.contains(LCDControl::DISPLAY_PRIORITY | LCDControl::WINDOW_DISPLAY_ENABLE)
    }

    // For sprites.
    pub fn is_large_sprites(&self) -> bool {
        self.lcd_control.contains(LCDControl::OBJ_SIZE)
    }

    // True: range 0x8000 - 0x8FFF for bg & window raw tile data
    pub fn lo_tile_data_select(&self) -> bool {
        self.lcd_control.contains(LCDControl::TILE_DATA_SELECT)
    }

    #[inline]
    pub fn can_access_vram(&self) -> bool {
        self.lcd_status.read_mode() != Mode::_3
    }

    #[inline]
    pub fn can_access_oam(&self) -> bool {
        !self.lcd_control.contains(LCDControl::OBJ_DISPLAY_ENABLE) ||
        (self.lcd_status.read_mode() == Mode::_0) ||
        (self.lcd_status.read_mode() == Mode::_1)
    }
                
    pub fn inc_lcdc_y(&mut self) {
        self.lcdc_y += 1;
        self.lcd_status.flags.set(LCDStatusFlags::COINCEDENCE_FLAG, self.lcdc_y == self.ly_compare);
    }

    pub fn set_lcdc_y(&mut self, val: u8) {
        self.lcdc_y = val;
        self.lcd_status.flags.set(LCDStatusFlags::COINCEDENCE_FLAG, self.lcdc_y == self.ly_compare);
    }

    pub fn write_mode(&mut self, mode: Mode) {
        self.lcd_status.write_mode(mode);
    }

    // TODO: improve these
    pub fn bg_tile_map_select(&self) -> bool {
        self.lcd_control.contains(LCDControl::BG_TILE_MAP_SELECT)
    }

    pub fn window_tile_map_select(&self) -> bool {
        self.lcd_control.contains(LCDControl::WINDOW_TILE_MAP_SELECT)
    }

    pub fn display_sprites(&self) -> bool {
        self.lcd_control.contains(LCDControl::OBJ_DISPLAY_ENABLE)
    }
}

// Reading
impl VideoRegs {

    pub fn read_lcdc_y(&self) -> u8 {
        self.lcdc_y
    }

    pub fn read_lcd_control(&self) -> u8 {
        self.lcd_control.bits()
    }

    pub fn read_status(&self) -> u8 {
        self.lcd_status.read()
    }
}

// Writing
impl VideoRegs {

    // Returns true if cycle count should be reset
    pub fn write_lcd_control(&mut self, val: u8) -> bool {
        let was_display_enabled = self.is_display_enabled();
        self.lcd_control = LCDControl::from_bits_truncate(val);
        let is_display_enabled = self.is_display_enabled();

        // Has display been toggled on/off?
        if is_display_enabled != was_display_enabled {
            if is_display_enabled { // ON
                self.lcd_status.write_mode(Mode::_2);
                return true;
            } else {                // OFF
                self.lcd_status.write_mode(Mode::_0);
                self.lcdc_y = 0;
            }
        }

        false
    }

    pub fn write_status(&mut self, val: u8) {
        self.lcd_status.write(val);
    }
}