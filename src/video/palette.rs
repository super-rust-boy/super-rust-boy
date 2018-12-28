use glium::texture::texture2d::Texture2d;
use glium;

type Pixel = (u8, u8, u8, u8);

// TODO: transparent sprite pixels
macro_rules! pixel {
    ( $x:expr ) => {
        {
            ($x as u8, $x as u8, $x as u8, 0xFF as u8) as Pixel
        }
    };
}

const WHITE: Pixel      = pixel!(0x80);//pixel!(0xFF);
const LIGHTGREY: Pixel  = pixel!(0x7F);
const DARKGREY: Pixel   = pixel!(0x3F);
const BLACK: Pixel      = pixel!(0x00);

pub struct BWPalette {
    pub data: u8,
}

pub trait Palette {
    fn get_pixel(&self, select: u8) -> Pixel;

    fn make_texture(&self, raw_tile: &[u8], display: &glium::backend::Facade) -> Texture2d {
        let mut texture_pixels: Vec<Vec<Pixel>> = Vec::new();
        let mut prev_byte = 0;
        for (x, d) in raw_tile.iter().cloned().enumerate() {
            // least significant byte of row
            if x % 2 == 0 {
                prev_byte = d;
            }
            // most significant byte of row
            else {
                let mut row = Vec::new();
                for i in 0..7 {
                    let (least_sig, most_sig) = ((prev_byte >> i) & 1_u8, (d >> i) & 1_u8);
                    let pixel_select = least_sig | (most_sig << 1);
                    let pixel = self.get_pixel(pixel_select);
                    row.push(pixel);
                }
                row.reverse();
                texture_pixels.push(row);
            }
        };

        // create texture
        Texture2d::with_mipmaps(display,
                                texture_pixels,
                                glium::texture::MipmapsOption::NoMipmap)
                   .unwrap()
    }
}

impl BWPalette {
    pub fn new() -> Self {
        BWPalette{
            data: 0
        }
    }

    pub fn read(&self) -> u8 {
        self.data
    }

    pub fn write(&mut self, in_data: u8) {
        self.data = in_data;
    }
}

impl Palette for BWPalette {
    // apply palette to input
    fn get_pixel(&self, select: u8) -> Pixel {
        let shift = (select & 0b11) * 2;
        let colour_index = (self.data >> shift) & 0b11;
        match colour_index {
            0b00 => WHITE,
            0b01 => LIGHTGREY,
            0b10 => DARKGREY,
            _    => BLACK,
        }
    }
}
