extern crate cc;

fn main() {
    cc::Build::new()
        .file("src/mmdecrypt.c")
        .compile("mmdecrypt");
}
