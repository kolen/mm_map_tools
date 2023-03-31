use nom::{
    combinator::verify, error::VerboseError, number::complete::le_u32, sequence::tuple, IResult,
};
use std::convert::TryInto;

#[derive(Debug)]
pub struct PrematureEndWhenSeekingTile {
    x: u32,
    y: u32,
    z: u32,
}

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
    pub fn from_contents(contents: Vec<u8>) -> Result<Self, String> {
        let result: IResult<_, _, VerboseError<_>> =
            tuple((verify(le_u32, |v| *v == 6), le_u32, le_u32, le_u32))(&*contents);
        let (_, (_, size_x, size_y, size_z)) =
            result.map_err(|e| format!("mps parse error: {:?}", e))?;

        let map_section = MapSection {
            size_x,
            size_y,
            size_z,
            contents,
        };
        // Eagerly check last tile
        map_section
            .tile_at_checked(size_x - 1, size_y - 1, size_z - 1)
            .map_err(|_| "Premature end of mps file")?;

        Ok(map_section)
    }

    pub fn tile_at(&self, x: u32, y: u32, z: u32) -> Tile {
        // PrematureEndWhenSeekingTile shouldn't happen as it's
        // checked in constructor
        self.tile_at_checked(x, y, z).unwrap()
    }

    fn tile_at_checked(&self, x: u32, y: u32, z: u32) -> Result<Tile, PrematureEndWhenSeekingTile> {
        assert!(x < self.size_x);
        assert!(y < self.size_y);
        assert!(z < self.size_z);
        let floor_bytes: usize = (self.size_x as usize) * (self.size_y as usize) * TILE_BYTES;
        let row_bytes: usize = (self.size_x as usize) * TILE_BYTES;
        let offset: usize =
            floor_bytes * (z as usize) + row_bytes * (y as usize) + TILE_BYTES * (x as usize);
        Ok(Tile {
            id: u16::from_le_bytes(
                self.tiles_data()[offset..offset + 2]
                    .try_into()
                    .map_err(|_| PrematureEndWhenSeekingTile { x, y, z })?,
            ),
        })
    }

    fn tiles_data(&self) -> &[u8] {
        &self.contents[TILES_OFFSET..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mm_compression::test_utils;
    #[test]
    #[ignore]
    fn test_from_contents() {
        let contents =
            test_utils::test_file_compressed_contents("Realms/Celtic/Forest/CFsec50.map");
        let map = MapSection::from_contents(contents).unwrap();
        assert_eq!(20, map.size_x);
        assert_eq!(20, map.size_y);
        assert_eq!(24, map.size_z);
        assert_eq!(0, map.tile_at(19, 19, 23).id);
    }
}
