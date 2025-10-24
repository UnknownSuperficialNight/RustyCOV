mod deps_download;
mod helpers;
mod lofty;
mod structs;
use std::collections::HashMap;
use std::thread::{JoinHandle, spawn};
use std::{path::PathBuf, process::Command};

use clap::{Arg, command};

use crate::{
    deps_download::download_and_extract_deps,
    lofty::embed_cover_image,
    structs::{Picked, RustyCov},
};

fn main() {
    let mut rusty_cov_global = RustyCov::new();

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
        .arg(Arg::new("cov_address").short('c').long("cov-address-url").num_args(1).value_name("cov_address_url").help("Address of the COV website to be opened on launch"))
        .get_matches();

    match download_and_extract_deps() {
        Ok(deps) => {
            rusty_cov_global.deps = Some(deps);
        }
        Err(e) => {
            eprintln!("Failed to download dependencies: {e}");
            return;
        }
    }

    // Extract the input string from the command line arguments.
    if let Some(raw) = matches.get_one::<String>("input_string") {
        rusty_cov_global.populate_from_input(raw);
    }

    if let Some(raw) = matches.get_one::<String>("cov_address") {
        rusty_cov_global.cov_address = Some(raw);
    }

    // If no files were found, exit.
    if rusty_cov_global.files.is_none() {
        eprintln!("No supported audio/video files were found exiting.");
        return;
    }

    // Store handles so we can wait for them later
    let mut handles: HashMap<usize, JoinHandle<()>> = HashMap::new();

    match rusty_cov_global.files {
        Some(mut list) if !list.is_empty() => {
            for (job_id, path) in list.drain(..).enumerate() {
                if let Some(picked) = run_covit(
                    rusty_cov_global.deps.as_ref().unwrap().covit.as_str(),
                    rusty_cov_global.cov_address.unwrap(),
                    &path,
                ) {
                    println!(
                        "Artist: {}\nTitle: {}\nDate: {}\nCover Type: {}\nImage Size: {} bytes\nDimensions: {}x{}\nBig Cover URL: {}\n",
                        picked.releaseInfo.artist,
                        picked.releaseInfo.title,
                        picked.releaseInfo.date,
                        picked.coverInfo.format,
                        picked.coverInfo.size,
                        picked.coverInfo.width,
                        picked.coverInfo.height,
                        picked.bigCoverUrl
                    );

                    let handle = spawn(move || {
                        // Embed the cover image
                        if let Err(e) = embed_cover_image(path, &picked.bigCoverUrl) {
                            eprintln!("Failed to embed cover: {}", e);
                        }
                    });
                    handles.insert(job_id, handle);
                } else {
                    println!("No cover info found for {:?}", path);
                }
            }
        }
        _ => eprintln!("No files were found or the input was invalid."),
    }

    let mut completed = 0usize;

    for (job_id, handle) in handles {
        match handle.join() {
            Ok(_) => {
                completed += 1;
            }
            Err(panic) => eprintln!("Job {} panicked: {:?}", job_id, panic),
        }
    }

    println!("Summary: {} job(s) finished.", completed);
}

/// Run covit and return the picked file.
fn run_covit(covit_path: &str, address: &str, input: &PathBuf) -> Option<Picked> {
    let output = Command::new(covit_path)
        .arg("--address")
        .arg(address)
        .arg("--input")
        .arg(input)
        .output()
        .expect("Failed to execute covit");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find the line starting with "Picked:"
    for line in stdout.lines() {
        if let Some(json) = line.strip_prefix("Picked: ") {
            // Try to deserialize
            if let Ok(picked) = serde_json::from_str::<Picked>(json) {
                return Some(picked);
            }
        }
    }
    None
}
