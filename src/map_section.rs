use std::convert::TryInto;

pub struct MapSection {
    pub size_x: u32,
    pub size_y: u32,
    pub size_z: u32,
    pub contents: Vec<u8>,
}

pub struct Tile {
    pub id: u16,
}

const TILE_BYTES: usize = 12;
const TILES_OFFSET: usize = 0x4c;

impl MapSection {
    pub fn from_contents(contents: Vec<u8>) -> Self {
        assert_eq!(
            6,
            u32::from_le_bytes(contents[0x0..0x4].try_into().unwrap())
        );
        MapSection {
            size_x: u32::from_le_bytes(contents[0x4..0x8].try_into().unwrap()),
            size_y: u32::from_le_bytes(contents[0x8..0xc].try_into().unwrap()),
            size_z: u32::from_le_bytes(contents[0xc..0x10].try_into().unwrap()),
            contents: contents,
        }
    }

    pub fn tile_at(&self, x: u32, y: u32, z: u32) -> Tile {
        assert!(x < self.size_x);
        assert!(y < self.size_y);
        assert!(z < self.size_z);
        let floor_bytes: usize = (self.size_x as usize) * (self.size_y as usize) * TILE_BYTES;
        let row_bytes: usize = (self.size_x as usize) * TILE_BYTES;
        let offset: usize =
            floor_bytes * (z as usize) + row_bytes * (y as usize) + TILE_BYTES * (x as usize);
        Tile {
            id: u16::from_le_bytes(self.tiles_data()[offset..offset + 2].try_into().unwrap()),
        }
    }

    fn tiles_data(&self) -> &[u8] {
        &self.contents[TILES_OFFSET..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils;
    #[test]
    #[ignore]
    fn test_from_contents() {
        let contents =
            test_utils::test_file_compressed_contents("Realms/Celtic/Forest/CFsec50.map");
        let map = MapSection::from_contents(contents);
        assert_eq!(20, map.size_x);
        assert_eq!(20, map.size_y);
        assert_eq!(24, map.size_z);
        assert_eq!(0, map.tile_at(19, 19, 23).id);
    }
}
