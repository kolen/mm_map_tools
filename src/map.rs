use byteorder::{ByteOrder, LittleEndian};

pub struct Map {
    size_x: u32,
    size_y: u32,
    size_z: u32,
    contents: Vec<u8>,
}

pub struct Tile {
    id: u16,
}

impl Map {
    pub fn from_contents(contents: Vec<u8>) -> Self {
        assert_eq!(6, LittleEndian::read_u32(&contents));
        Map {
            size_x: LittleEndian::read_u32(&contents[0x4..]),
            size_y: LittleEndian::read_u32(&contents[0x8..]),
            size_z: LittleEndian::read_u32(&contents[0xc..]),
            contents: contents,
        }
    }

    pub fn tile_at(&self, x: u32, y: u32, z: u32) -> Tile {
        assert!(x < self.size_x);
        assert!(y < self.size_y);
        assert!(z < self.size_z);
        let offset: usize = self.size_z as usize * self.size_y as usize * 6 * z as usize
            + self.size_y as usize * 6 * y as usize
            + x as usize * 6;
        Tile {
            id: LittleEndian::read_u16(&self.tiles_data()[offset..]),
        }
    }

    fn tiles_data(&self) -> &[u8] {
        &self.contents[0x4c..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils;
    #[test]
    fn test_from_contents() {
        let contents =
            test_utils::test_file_compressed_contents("Realms/Celtic/Forest/CFsec50.map");
        let map = Map::from_contents(contents);
        assert_eq!(20, map.size_x);
        assert_eq!(20, map.size_y);
        assert_eq!(24, map.size_z);
        assert_eq!(878, map.tile_at(10, 10, 20).id);
    }
}
