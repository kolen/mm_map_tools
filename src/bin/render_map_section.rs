extern crate mm_map_tools;
use mm_map_tools::container::read_decompressed;
use mm_map_tools::map_section::MapSection;
use mm_map_tools::render::{render_map_section, RenderOptions};
use mm_map_tools::sprite_file::SpriteFile;
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let map_section_path = Path::new(&args[1]);
    let sprites_path = map_section_path
        .parent()
        .expect("Invalid map section path")
        .join(Path::new("Terrain.spr"));

    println!("{:?}", sprites_path);

    let map_section = MapSection::from_contents(
        read_decompressed(map_section_path).expect("Couldn't deobfuscate map section"),
    )
    .expect("Couldn't parse map section");
    let sprites = SpriteFile::parse(File::open(sprites_path).unwrap());
    let image = render_map_section(&map_section, &sprites, &RenderOptions::default());
    image.save(&args[2]).unwrap();
}
