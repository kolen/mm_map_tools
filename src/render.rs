use image;
use map_section::MapSection;
use nalgebra::{Matrix, Matrix2, Matrix2x3, MatrixArray, Vector2, Vector3, U1, U3};
use sprite_file::SpriteFile;

const TILE_W: i32 = 64;
const TILE_H: i32 = 32;
const TILE_Z_OFFSET: i32 = 48 - 32; // TODO: figure out

type TileCoordinates = Matrix<i32, U3, U1, MatrixArray<i32, U3, U1>>;

fn rotate_tile_coordinates(
    source: TileCoordinates,
    size_x: u32,
    size_y: u32,
    rotation_number: u8,
) -> TileCoordinates {
    let tilexy = Vector2::new(source.x, source.y);
    let rotation = match rotation_number {
        0 => Matrix2::new(1, 0, 0, 1),
        1 => Matrix2::new(0, -1, 1, 0),
        2 => Matrix2::new(-1, 0, 0, -1),
        3 => Matrix2::new(0, 1, -1, 0),
        _ => panic!("Invalid rotation number"),
    };
    let translation: Vector2<i32> = match rotation_number {
        0 => Vector2::new(0, 0),
        1 => Vector2::new(size_x as i32 - 1, 0),
        2 => Vector2::new(size_x as i32 - 1, size_y as i32 - 1),
        3 => Vector2::new(0, size_y as i32 - 1),
        _ => panic!("Invalid rotation number"),
    };
    let tilexy2 = rotation * tilexy + translation;
    debug_assert!(tilexy2.x >= 0);
    debug_assert!(tilexy2.y >= 0);
    debug_assert!(tilexy2.x < size_x as i32);
    debug_assert!(tilexy2.x < size_y as i32);
    Vector3::new(tilexy2.x, tilexy2.y, source.z)
}

fn project(tile_coordinates: TileCoordinates) -> Vector2<i32> {
    #[rustfmt::skip]
    let projection: Matrix2x3<i32> = Matrix2x3::new(
        TILE_W / 2,   TILE_W / 2,   0,
        TILE_H / 2, -(TILE_H / 2), -TILE_Z_OFFSET,
    );
    let base_point: Vector2<i32> = Vector2::new(0, 1024);

    base_point + projection * tile_coordinates
}

fn map_rendering_order<'a>(map_section: &'a MapSection) -> impl Iterator<Item = TileCoordinates> {
    let (sx, sy, sz) = (map_section.size_x, map_section.size_y, map_section.size_z);
    (0..sz).flat_map(move |z| {
        (0..sx).flat_map(move |x| {
            (0..sy)
                .rev()
                .map(move |y| Vector3::new(x as i32, y as i32, z as i32))
        })
    })
}

fn blit(destination: &mut image::RgbaImage, source: &image::RgbaImage, pos: Vector2<i32>) {
    for x in 0..source.width() as i32 {
        for y in 0..source.height() as i32 {
            let dest_x = x + pos.x;
            let dest_y = y + pos.y;
            if dest_x >= 0
                && dest_x < (destination.width() as i32)
                && dest_y >= 0
                && dest_y < (destination.height() as i32)
            {
                let src_pixel = source.get_pixel(x as u32, y as u32);
                if src_pixel[3] != 0 {
                    destination.put_pixel(
                        (x + pos.x) as u32,
                        (y + pos.y) as u32,
                        src_pixel.clone(),
                    );
                }
            }
        }
    }
}

fn draw_tile(
    canvas: &mut image::RgbaImage,
    sprites: &SpriteFile,
    tile_coordinates: TileCoordinates,
    tile_id: u16,
) {
    if tile_id == 0xffff || tile_id == 0x0000 {
        return;
    }
    let proj_tile_coordinates = project(tile_coordinates);
    let sprite = &sprites.frames[tile_id as usize];
    let target_coordinates = proj_tile_coordinates - Vector2::new(sprite.center_x, sprite.center_y);

    blit(
        canvas,
        &sprites.frames[tile_id as usize].image,
        target_coordinates,
    )
}

pub fn render_map_section(map_section: &MapSection, sprites: &SpriteFile) -> image::RgbaImage {
    let mut canvas = image::RgbaImage::new(2048, 2048);
    for tile_coordinates in map_rendering_order(map_section) {
        draw_tile(
            &mut canvas,
            &sprites,
            tile_coordinates,
            map_section
                .tile_at(
                    tile_coordinates.x as u32,
                    map_section.size_y - (tile_coordinates.y as u32) - 1,
                    tile_coordinates.z as u32,
                ).id,
        );
    }
    canvas
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_section::MapSection;
    use sprite_file::SpriteFile;
    use std::fs::File;
    use test_utils::*;

    #[test]
    fn test_render_map_section() {
        let map_section = MapSection::from_contents(test_file_compressed_contents(
            "Realms/Celtic/Forest/CFSec10.map",
        ));
        let sprites = SpriteFile::parse(
            File::open(test_file_path("Realms/Celtic/Forest/Terrain.spr")).unwrap(),
        );
        let canvas = render_map_section(&map_section, &sprites);
        canvas.save("/tmp/map.png").unwrap();
    }
}
