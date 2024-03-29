use crate::{render_map_section, MapSection, RenderOptions};
use anyhow::{anyhow, Result};
use mm_compression::read_decompressed;
use mm_file_formats::sprites::Sprites;
use std::fs::File;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub struct RendererCache {
    section_path: PathBuf,
    sprites_path: PathBuf,
    map_section: MapSection,
    sprites: Sprites,
}

pub struct Renderer {
    mm_path: PathBuf,
    cache: RwLock<Option<RendererCache>>,
}

pub fn load_sprites_and_map_section_cached<
    L1: Fn(&Path) -> Result<MapSection>,
    L2: Fn(&Path) -> Result<Sprites>,
>(
    cache: Option<RendererCache>,
    section_path: &Path,
    sprites_path: &Path,
    load_section: L1,
    load_sprites: L2,
) -> Result<RendererCache> {
    match cache {
        None => Ok(RendererCache {
            section_path: section_path.to_path_buf(),
            sprites_path: sprites_path.to_path_buf(),
            map_section: load_section(section_path)?,
            sprites: load_sprites(sprites_path)?,
        }),
        Some(cache) => {
            let new_map_section = if cache.section_path == section_path {
                cache.map_section
            } else {
                load_section(section_path)?
            };
            let new_sprites = if cache.sprites_path == sprites_path {
                cache.sprites
            } else {
                load_sprites(sprites_path)?
            };
            Ok(RendererCache {
                section_path: section_path.to_path_buf(),
                sprites_path: sprites_path.to_path_buf(),
                map_section: new_map_section,
                sprites: new_sprites,
            })
        }
    }
}

impl Renderer {
    pub fn new(mm_path: &Path) -> Self {
        Renderer {
            mm_path: mm_path.to_path_buf(),
            cache: RwLock::new(None),
        }
    }

    fn section_path(&self, map_group: &str, map_section: &str) -> PathBuf {
        self.mm_path
            .join("Realms")
            .join(&map_group)
            .join(&map_section)
            .with_extension("map")
    }

    pub fn render(
        &self,
        map_group: &str,
        map_section: &str,
        options: &RenderOptions,
    ) -> Result<image::RgbaImage> {
        let map_section_path_1 = self.section_path(&map_group, &map_section);
        let sprites_path = map_section_path_1
            .parent()
            .ok_or(anyhow!("Can't get parent of map section path"))?
            .join(Path::new("Terrain.spr"));

        let mut cache_writer = self
            .cache
            .write()
            .map_err(|e| anyhow!("Can't unlock cache for writing: {}", e))?;
        let old_cache_contents = mem::replace(&mut *cache_writer, None);
        let new_cache_contents = load_sprites_and_map_section_cached(
            old_cache_contents,
            &map_section_path_1,
            &sprites_path,
            |map_section_path| {
                eprintln!("Loading map section {:?}", &map_section_path);
                Ok(MapSection::from_contents(read_decompressed(
                    map_section_path,
                )?)?)
            },
            |sprites_path| {
                eprintln!("Loading sprites {:?}", &sprites_path);
                Ok(Sprites::parse(File::open(sprites_path)?))
            },
        )?;

        eprintln!("Rendering {}/{}", map_group, map_section);
        let image = render_map_section(
            &new_cache_contents.map_section,
            &new_cache_contents.sprites,
            options,
        );
        *cache_writer = Some(new_cache_contents);
        Ok(image)
    }
}
