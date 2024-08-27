use std::path::PathBuf;

use clap::Parser;
use dmm_lite::parse_map_multithreaded;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    files: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    for file in args.files {
        if let Some(s) = file.extension().map(|s| s.to_string_lossy()) {
            if s != "dmm" {
                continue;
            }
        } else {
            continue;
        }

        let string = std::fs::read_to_string(&file)?;
        match parse_map_multithreaded(&string) {
            Ok((info, (prefabs, blocks))) => {
                println!(
                    "\x1b[32mSuccesfully parsed {file:#?} - TGM? {} - {} prefabs, {} blocks\x1b[0m",
                    info.is_tgm,
                    prefabs.len(),
                    blocks.len()
                );
            }
            Err(e) => {
                eprintln!("\x1b[31mFAILED Parsing {file:#?}\x1b[0m");
                e.debug_print(&string);
            }
        }
    }

    Ok(())
}
