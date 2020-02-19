use super::{
    VRAM,
    sprite::Sprite,
    super::types::Colour,
    super::regs::VideoRegs
};

const TILE_MAP_WIDTH: usize = 32;
const SCREEN_WIDTH: usize = 160;

impl VRAM {
    pub fn draw_line_gb(&mut self, target: &mut [u8], regs: &VideoRegs) {    // TODO: use external type here.
        let y = regs.read_lcdc_y();
        let target_start = (y as usize) * SCREEN_WIDTH;
        //println!("Draw line {}", y);

        // Rebuild caches
        if self.map_cache_0_dirty {
            self.construct_map_cache_0(regs);
        }
        if self.map_cache_1_dirty {
            self.construct_map_cache_1(regs);
        }

        // Find objects
        let objects = self.ref_objects_for_line(y, regs);
        let mut sprite_pixels = [SpritePixel::None; 160];

        self.render_sprites_to_line(&mut sprite_pixels, &objects, y, regs.is_large_sprites());

        for (x, i) in target.chunks_mut(4).skip(target_start).take(SCREEN_WIDTH).enumerate() {
            match sprite_pixels[x] {
                SpritePixel::Hi(c) => write_pixel(i, c),
                SpritePixel::Lo(c) => if let Some(px) = self.window_pixel(x as u8, y, regs) {
                    match px {
                        BGPixel::Zero(_) => write_pixel(i, c),
                        BGPixel::NonZero(win) => write_pixel(i, win),
                    }
                } else {
                    match self.background_pixel(x as u8, y, regs) {
                        BGPixel::Zero(_) => write_pixel(i, c),
                        BGPixel::NonZero(bg) => write_pixel(i, bg),
                    }
                },
                SpritePixel::None => if let Some(px) = self.window_pixel(x as u8, y, regs) {
                    match px {
                        BGPixel::Zero(win) => write_pixel(i, win),
                        BGPixel::NonZero(win) => write_pixel(i, win),
                    }
                } else {
                    match self.background_pixel(x as u8, y, regs) {
                        BGPixel::Zero(bg) => write_pixel(i, bg),
                        BGPixel::NonZero(bg) => write_pixel(i, bg),
                    }
                }
            }
        }
    }

    fn render_sprites_to_line(&self, line: &mut [SpritePixel], objects: &[&Sprite], y: u8, large: bool) {
        for o in objects {
            let sprite_y = y + 16 - o.y;
            let (tile_num_offset, tile_y) = match (large, sprite_y < 8, o.flip_y()) {
                (false, true, false)    => (0_u8, sprite_y),
                (false, true, true)     => (0_u8, 7 - sprite_y),
                (true, true, false)     => (0_u8, sprite_y),
                (true, false, false)    => (1_u8, sprite_y - 8),
                (true, true, true)      => (1_u8, 7 - sprite_y),
                (true, false, true)     => (0_u8, 15 - sprite_y),
                _ => unreachable!("Cannot have small sprites with sprite_y >= 8")
            };
            let tile = self.ref_tile(o.tile_num.wrapping_add(tile_num_offset) as usize);

            let start_x = (o.x as isize) - 8;
            for x_offset in 0..8 {
                let x = start_x + x_offset;
                if x >= 0 && x < 160 {
                    let tile_x = if o.flip_x() {7 - x_offset} else {x_offset};
                    let texel = tile.get_texel(tile_x as usize, tile_y as usize);
                    if texel != 0 {
                        let pixel = if o.palette_0() {self.get_obj_0_colour(texel)} else {self.get_obj_1_colour(texel)};
                        line[x as usize] = if o.is_above_bg() {
                            SpritePixel::Hi(pixel)
                        } else {
                            SpritePixel::Lo(pixel)
                        };
                    }
                }
                // For x
            }
            // For objects
        }
    }

    #[inline]
    fn window_pixel(&self, x: u8, y: u8, regs: &VideoRegs) -> Option<BGPixel> {
        if regs.get_window_enable() && (x + 7 >= regs.window_x) && (y >= regs.window_y) {
            let win_x = (x + 7 - regs.window_x) as usize;
            let win_y = (y - regs.window_y) as usize;
            let win_texel = self.ref_window(regs)[win_y][win_x];
            Some(if win_texel == 0 {
                BGPixel::Zero(self.get_bg_colour(win_texel))
            } else {
                BGPixel::NonZero(self.get_bg_colour(win_texel))
            })
        } else {
            None
        }
    }

    #[inline]
    fn background_pixel(&self, x: u8, y: u8, regs: &VideoRegs) -> BGPixel {
        if regs.get_background_priority() {
            let bg_x = regs.scroll_x.wrapping_add(x) as usize;
            let bg_y = regs.scroll_y.wrapping_add(y) as usize;
            let bg_texel = self.ref_background(regs)[bg_y][bg_x];
            if bg_texel == 0 {
                BGPixel::Zero(self.get_bg_colour(bg_texel))
            } else {
                BGPixel::NonZero(self.get_bg_colour(bg_texel))
            }
        } else {
            BGPixel::Zero(Colour::zero())
        }
    }
}

impl VRAM {
    fn construct_map_cache_0(&mut self, regs: &VideoRegs) {
        for (i, tile_num) in self.tile_map_0.iter().enumerate() {
            // TODO: iterate over tile
            let base_y = (i / 32) << 3;
            let base_x = (i % 32) << 3;
            for y in 0..8 {
                for x in 0..8 {
                    // TODO: attrs
                    let tile_index = if regs.lo_tile_data_select() {
                        *tile_num as usize
                    } else {
                        let signed = *tile_num as i8;
                        (256 + (signed as isize)) as usize
                    };
                    let tex = self.tile_mem.ref_tile(tile_index).get_texel(x, y);
                    self.map_cache_0[base_y + y][base_x + x] = tex;
                    //println!("{}, {}: {}", base_x + x, base_y + y, tex)
                }
            }
        }

        self.map_cache_0_dirty = false;
    }

    fn construct_map_cache_1(&mut self, regs: &VideoRegs) {
        for (i, tile_num) in self.tile_map_1.iter().enumerate() {
            // TODO: iterate over tile
            let base_y = (i / 32) << 3;
            let base_x = (i % 32) << 3;
            for y in 0..8 {
                for x in 0..8 {
                    // TODO: attrs
                    let tile_index = if regs.lo_tile_data_select() {
                        *tile_num as usize
                    } else {
                        let signed = *tile_num as i8;
                        (256 + (signed as isize)) as usize
                    };
                    let tex = self.tile_mem.ref_tile(tile_index).get_texel(x, y);
                    self.map_cache_1[base_y + y][base_x + x] = tex;
                }
            }
        }

        self.map_cache_1_dirty = false;
    }
}

#[derive(Clone, Copy)]
enum SpritePixel {
    Hi(Colour), // High priority
    Lo(Colour), // Low priority
    None
}

enum BGPixel {
    NonZero(Colour),    // Colour 1-3
    Zero(Colour),       // Zero colour (draw LO sprites above this)
}

#[inline]
fn write_pixel(output: &mut [u8], colour: Colour) {
    output[0] = colour.r;
    output[1] = colour.g;
    output[2] = colour.b;
}