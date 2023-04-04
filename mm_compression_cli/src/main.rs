use clap::{Parser, Subcommand};
use mm_compression;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    about = "A tools to manipulate compression and obfuscation formats of Magic & Mayhem files"
)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Decompress {
        source: PathBuf,
        destination: Option<PathBuf>,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Decompress {
            source,
            destination,
        } => {
            // TODO: use streaming
            let mut destination_file: Box<dyn io::Write> = match destination {
                Some(filename) => Box::new(fs::File::create(filename).unwrap()),
                None => Box::new(io::stdout()),
            };

            let decompressed = mm_compression::read_decompressed(source).unwrap();
            destination_file.write_all(&decompressed).unwrap();
        }
    }
}
