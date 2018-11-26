use byteorder::{ByteOrder, LittleEndian};

pub struct Map {
    size_x: u32,
    size_y: u32,
    size_z: u32,
    contents: Vec<u8>,
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
    }
}
