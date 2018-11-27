extern crate mm_map_tools;
use mm_map_tools::decompress::read_decompressed;
use mm_map_tools::map_section::MapSection;
use mm_map_tools::render::render_map_section;
use mm_map_tools::sprite_file::SpriteFile;
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let map_section_path = Path::new(&args[1]);
    let sprites_path = map_section_path
        .parent()
        .unwrap()
        .join(Path::new("Terrain.spr"));

    println!("{:?}", sprites_path);

    let map_section = MapSection::from_contents(read_decompressed(map_section_path).unwrap());
    let sprites = SpriteFile::parse(File::open(sprites_path).unwrap());

    let image = render_map_section(&map_section, &sprites);
    image.save(&args[2]).unwrap();
}
