// Dealing with sprites.
use bitflags::bitflags;

use crate::mem::MemDevice;

const SPRITE_SMALL_HEIGHT: u8 = 8;
const SPRITE_LARGE_HEIGHT: u8 = 16;

bitflags! {
    #[derive(Default)]
    pub struct SpriteFlags: u8 {
        const PRIORITY  = bit!(7);
        const Y_FLIP    = bit!(6);
        const X_FLIP    = bit!(5);
        const PALETTE   = bit!(4);
        const VRAM_BANK = bit!(3);
        const CGB_PAL_2 = bit!(2);
        const CGB_PAL_1 = bit!(1);
        const CGB_PAL_0 = bit!(0);
    }
}

#[derive(Clone)]
pub struct Sprite {
    pub y:          u8,
    pub x:          u8,
    pub tile_num:   u8,
    pub flags:      SpriteFlags
}

impl Sprite {
    pub fn new() -> Self {
        Sprite {
            y:          0,
            x:          0,
            tile_num:   0,
            flags:      SpriteFlags::default()
        }
    }

    pub fn is_above_bg(&self) -> bool {
        !self.flags.contains(SpriteFlags::PRIORITY)
    }

    pub fn palette_0(&self) -> bool {
        !self.flags.contains(SpriteFlags::PALETTE)
    }

    pub fn flip_x(&self) -> bool {
        self.flags.contains(SpriteFlags::X_FLIP)
    }

    pub fn flip_y(&self) -> bool {
        self.flags.contains(SpriteFlags::Y_FLIP)
    }
}

pub struct ObjectMem {
    objects:    Vec<Sprite>,
}

impl ObjectMem {
    pub fn new() -> Self {
        ObjectMem {
            objects: vec![Sprite::new(); 40],
        }
    }

    pub fn ref_objects_for_line<'a>(&'a self, y: u8, large: bool) -> Vec<&'a Sprite> {
        let y_upper = y + 16;
        let y_lower = y_upper - if large {SPRITE_LARGE_HEIGHT} else {SPRITE_SMALL_HEIGHT};
        self.objects.iter().filter(|o| {
            (o.y > y_lower) && (o.y <= y_upper)
        }).collect::<Vec<_>>()
    }
}

// Expects a loc range from 0 -> 0x9F
impl MemDevice for ObjectMem {
    fn read(&self, loc: u16) -> u8 {
        let index = (loc / 4) as usize;

        match loc % 4 {
            0 => self.objects[index].y,
            1 => self.objects[index].x,
            2 => self.objects[index].tile_num,
            _ => self.objects[index].flags.bits()
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        let index = (loc / 4) as usize;

        match loc % 4 {
            0 => self.objects[index].y = val,
            1 => self.objects[index].x = val,
            2 => self.objects[index].tile_num = val,
            _ => self.objects[index].flags = SpriteFlags::from_bits_truncate(val)
        }
    }
}