# mm_map_tools

Collection of things to read [Magic & Mayhem](https://en.wikipedia.org/wiki/Magic_and_Mayhem) file formats.

Not a serious project for now. What can it do:

* Read `.spr` sprite sheets (such as `Terrain.spr`)
* Read and render `.map` map fragments (only basic features)

Most format descriptions are from [sau](https://github.com/saniv/sau/) project, including decompression/deobfuscation code.

## `render_map_section` binary

Renders map section to image file.

```
render_map_section input_map_section.map output.png
```

## Running tests

By default, tests requiring original Magic & Mayhem files are ignored with `#[ignore]`, to run them, specify M&M path in `MM_PATH` env variable and use `cargo test -- --ignored`.
