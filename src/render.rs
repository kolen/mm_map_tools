use image;
use map_section::MapSection;
use nalgebra::{Matrix, Matrix2x3, MatrixArray, Vector2, Vector3, U1, U3};
use sprite_file::SpriteFile;

const TILE_W: i32 = 64;
const TILE_H: i32 = 48;
const TILE_Z_OFFSET: i32 = 10;

type TileCoordinates = Matrix<i32, U3, U1, MatrixArray<i32, U3, U1>>;

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
    let target_coordinates = project(tile_coordinates);

    if tile_id == 0xffff || tile_id == 0x0000 {
        return;
    }
    println!("{:?}", target_coordinates);

    blit(
        canvas,
        &sprites.frames[tile_id as usize].image,
        Vector2::new(target_coordinates.x, target_coordinates.y),
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
                    tile_coordinates.y as u32,
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
