use glium::texture::texture2d::Texture2d;

use std::collections::HashMap;

pub type Hash = u32;

pub struct TexCache {
    map:    HashMap<u32, Texture2d>,
}

impl TexCache {
    pub fn new() -> Self {
        TexCache {
            map: HashMap::new(),
        }
    }

    // Make hash
    pub fn make_hash(tile_loc: usize, palette: u8) -> Hash {
        ((palette as u32) << 24) | (tile_loc as u32)
    }


    // Clearing in the event tile data changes
    pub fn clear(&mut self, tile_loc: usize, palette: u8) {
        self.map.remove(&Self::make_hash(tile_loc, palette));
    }

    // Clearing in the case the bg palette changes
    pub fn clear_all(&mut self) {
        self.map.clear();
    }

    // Add to map
    pub fn insert(&mut self, hash: Hash, tex: Texture2d) {
        self.map.insert(hash, tex);
    }

    // Retrieve from map
    pub fn get(&self, hash: &Hash) -> Option<&Texture2d> {
        self.map.get(hash)
    }

    pub fn contains_key(&self, hash: &Hash) -> bool {
        self.map.contains_key(hash)
    }
}
