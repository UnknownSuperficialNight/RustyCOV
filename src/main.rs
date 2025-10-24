mod deps_download;
mod helpers;
mod structs;

use clap::{Arg, command};

use crate::{deps_download::download_and_extract_deps, structs::RustyCov};

fn main() {
    let mut rusty_cov_global = RustyCov::new();

    match download_and_extract_deps() {
        Ok(deps) => {
            rusty_cov_global.deps = Some(deps);
        }
        Err(e) => {
            eprintln!("Failed to download dependencies: {e}");
            return;
        }
    }

    let matches = command!()
        .arg(
            Arg::new("input_string")
                .short('i')
                .long("input")
                .num_args(1)
                .value_name("input-string")
                .help("Input directory or file to process").long_help("Input a directory that will be recursively processed or a single file to process")
                .required(true),
        )
        .get_matches();

    // Extract the input string from the command line arguments.
    if let Some(raw) = matches.get_one::<String>("input_string") {
        rusty_cov_global.populate_from_input(raw);
    }

    // If no files were found, exit.
    if rusty_cov_global.files.is_none() {
        eprintln!("No supported audio/video files were found exiting.");
        return;
    }

    match &rusty_cov_global.files {
        Some(list) if !list.is_empty() => {
            for (path, fmt) in list {
                println!("{} [{:?}]", path.display(), fmt);
                // Insert your per‑file processing here.
            }
        }
        _ => eprintln!("No files were found or the input was invalid."),
    }
}
