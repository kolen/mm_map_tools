use image;
use crate::map_section::MapSection;
use nalgebra::{ArrayStorage, Matrix, Matrix2x3, Vector2, Vector3, U1, U3};
use crate::sprite_file::SpriteFile;
use std::cmp;

pub mod utils;

const TILE_W: i32 = 64;
const TILE_H: i32 = 32;
const TILE_HALF_W: i32 = TILE_W / 2;
const TILE_HALF_H: i32 = TILE_H / 2;
const TILE_Z_OFFSET: i32 = 48 - 32; // TODO: figure out

type TileCoordinates = Matrix<i32, U3, U1, ArrayStorage<i32, U3, U1>>;

#[derive(Debug)]
struct CanvasSize {
    size: Vector2<u32>,
    center: Vector2<i32>,
}

pub struct RenderOptions {
    pub max_layer: u32,
}

impl RenderOptions {
    pub fn default() -> Self {
        RenderOptions { max_layer: 255 }
    }
}

impl CanvasSize {
    fn for_map_section(map_section: &MapSection) -> Self {
        let size = Vector2::new(
            (map_section.size_x + map_section.size_y) * TILE_HALF_W as u32,
            (map_section.size_x + map_section.size_y) * TILE_HALF_H as u32
                + map_section.size_z * TILE_Z_OFFSET as u32,
        );
        let center = Vector2::new(
            map_section.size_y as i32 * TILE_HALF_W,
            map_section.size_z as i32 * TILE_Z_OFFSET,
        );
        CanvasSize { size, center }
    }
}

fn project(tile_coordinates: TileCoordinates) -> Vector2<i32> {
    /*
                     /|\ z
                      |
                      |
                      o. . . . . x'
                    -`:`-
                  -`  :  `-
              y \/    :y'  \/ x
    */
    #[rustfmt::skip]
    let projection: Matrix2x3<i32> = Matrix2x3::new(
        TILE_HALF_W, -TILE_HALF_W,  0,
        TILE_HALF_H,  TILE_HALF_H, -TILE_Z_OFFSET,
    );
    projection * tile_coordinates
}

fn map_rendering_order(
    map_section: &MapSection,
    max_z: u32,
) -> impl Iterator<Item = TileCoordinates> {
    let (sx, sy, sz) = (
        map_section.size_x,
        map_section.size_y,
        cmp::min(map_section.size_z, max_z),
    );
    (0..sz).flat_map(move |z| {
        (0..sx).flat_map(move |x| (0..sy).map(move |y| Vector3::new(x as i32, y as i32, z as i32)))
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
    origin: Vector2<i32>,
) {
    if tile_id == 0xffff || tile_id == 0x0000 {
        return;
    }
    let proj_tile_coordinates = project(tile_coordinates);
    let sprite = &sprites.frames[tile_id as usize];
    let target_coordinates =
        proj_tile_coordinates - Vector2::new(sprite.center_x, sprite.center_y) + origin;

    blit(
        canvas,
        &sprites.frames[tile_id as usize].image,
        target_coordinates,
    )
}

pub fn render_map_section(
    map_section: &MapSection,
    sprites: &SpriteFile,
    options: &RenderOptions,
) -> image::RgbaImage {
    let canvas_size = CanvasSize::for_map_section(map_section);
    let mut canvas = image::RgbaImage::new(canvas_size.size.x, canvas_size.size.y);
    for tile_coordinates in map_rendering_order(map_section, options.max_layer) {
        draw_tile(
            &mut canvas,
            &sprites,
            tile_coordinates,
            map_section
                .tile_at(
                    tile_coordinates.x as u32,
                    tile_coordinates.y as u32,
                    tile_coordinates.z as u32,
                )
                .id,
            canvas_size.center,
        );
    }
    canvas
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_section::MapSection;
    use crate::sprite_file::SpriteFile;
    use std::fs::File;
    use crate::test_utils::*;

    #[test]
    #[ignore]
    fn test_render_map_section() {
        let map_section = MapSection::from_contents(test_file_compressed_contents(
            "Realms/Celtic/Forest/CFSec10.map",
        ))
        .unwrap();
        let sprites = SpriteFile::parse(
            File::open(test_file_path("Realms/Celtic/Forest/Terrain.spr")).unwrap(),
        );
        render_map_section(&map_section, &sprites, &RenderOptions::default());
    }
}
