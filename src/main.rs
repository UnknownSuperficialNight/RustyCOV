mod deps_download;
mod helpers;
mod image;
mod lofty;
mod structs;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::{JoinHandle, spawn};
use std::{path::PathBuf, process::Command};

use clap::{Arg, ArgAction, command};

use crate::{
    deps_download::download_and_extract_deps,
    lofty::embed_cover_image,
    structs::{Picked, RustyCov},
};

fn main() {
    let mut rusty_cov_global = RustyCov::new();

    let mut cmd = command!()
        .arg(
            Arg::new("input_string")
                .short('i')
                .long("input")
                .num_args(1)
                .value_name("input-string")
                .help("Input directory or file to process").long_help("Input a directory that will be recursively processed or a single file to process")
                .required(true),
        )
        .arg(Arg::new("cov_address").short('c').long("cov-address-url").num_args(1).value_name("cov_address_url").help("Address of the COV website to be opened on launch"));

    // Conditionally add arguments
    #[cfg(feature = "jpeg-opt")]
    {
        cmd = cmd
            .arg(
                Arg::new("convert_png_to_jpg")
                    .short('j')
                    .long("convert-png-to-jpg")
                    .help("Convert PNG to JPG")
                    .long_help("If a PNG is selected, convert it to JPG to save space")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("jpeg_optimise")
                    .long("jpeg-optimise")
                    .help("Optimise JPEG images")
                    .action(ArgAction::SetTrue),
            );
    }

    #[cfg(feature = "png-opt")]
    {
        cmd = cmd.arg(
            Arg::new("png_opt")
                .long("png-opt")
                .help("Optimise PNG images")
                .action(ArgAction::SetTrue),
        );
    }

    let matches = cmd.get_matches();

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

    // For each flag, create an Arc<AtomicBool> and set its value
    #[cfg(feature = "jpeg-opt")]
    let convert_png_to_jpg = Arc::new(AtomicBool::new(matches.get_flag("convert_png_to_jpg")));

    #[cfg(not(feature = "jpeg-opt"))]
    let convert_png_to_jpg = Arc::new(AtomicBool::new(false));

    #[cfg(feature = "jpeg-opt")]
    let jpeg_optimise = Arc::new(AtomicBool::new(matches.get_flag("jpeg_optimise")));

    #[cfg(not(feature = "jpeg-opt"))]
    let jpeg_optimise = Arc::new(AtomicBool::new(false));

    #[cfg(feature = "png-opt")]
    let png_opt = Arc::new(AtomicBool::new(matches.get_flag("png_opt")));

    #[cfg(not(feature = "png-opt"))]
    let png_opt = Arc::new(AtomicBool::new(false));

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
                        picked.release_info.artist,
                        picked.release_info.title,
                        picked.release_info.date,
                        picked.cover_info.format,
                        picked.cover_info.size,
                        picked.cover_info.width,
                        picked.cover_info.height,
                        picked.big_cover_url
                    );

                    let convert_png_to_jpg = Arc::clone(&convert_png_to_jpg);
                    let jpeg_optimise = Arc::clone(&jpeg_optimise);
                    let png_opt = Arc::clone(&png_opt);

                    let handle = spawn(move || {
                        // Embed the cover image
                        if let Err(e) = embed_cover_image(
                            path,
                            &picked.big_cover_url,
                            convert_png_to_jpg,
                            jpeg_optimise,
                            png_opt,
                        ) {
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
    use std::process::Command;

    // First attempt: run covit normally
    let output = Command::new(covit_path)
        .arg("--address")
        .arg(address)
        .arg("--input")
        .arg(input)
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check if output only contains "Listening: <number>"
    if stdout.lines().all(|line| line.starts_with("Listening:")) && stderr.trim().is_empty() {
        println!("User closed the tab");
        return None; // User closed the browser tab
    }

    let picked = parse_covit_output(output.stdout);

    // If picked is not None, return it
    if let Some(picked) = picked {
        return Some(picked);
    }

    // Fallback: parse file name for artist and title
    let file_stem = input.file_stem()?.to_str()?;
    let (artist_opt, title_opt) = parse_file_name(file_stem);

    // Only retry if we have at least a title
    let title = match title_opt {
        Some(ref t) if !t.is_empty() => t,
        _ => return None,
    };

    // Second attempt: run covit with --query-artist and --query-album
    let output = run_covit_query(covit_path, address, title, artist_opt)?;
    parse_covit_output(output.stdout)
}

fn parse_covit_output(stdout: Vec<u8>) -> Option<Picked> {
    let stdout = String::from_utf8_lossy(&stdout);

    for line in stdout.lines() {
        if let Some(json) = line.strip_prefix("Picked: ")
            && let Ok(picked) = serde_json::from_str::<Picked>(json)
        {
            return Some(picked);
        }
    }

    None
}

fn run_covit_query(
    covit_path: &str,
    address: &str,
    title: &str,
    artist_opt: Option<String>,
) -> Option<std::process::Output> {
    let mut cmd = Command::new(covit_path);
    cmd.arg("--address")
        .arg(address)
        .arg("--query-album")
        .arg(title);

    if let Some(ref artist) = artist_opt
        && !artist.is_empty()
    {
        cmd.arg("--query-artist").arg(artist);
    }

    cmd.output().ok()
}

fn parse_file_name(file_stem: &str) -> (Option<String>, Option<String>) {
    let delimiters = [" - ", " – ", " — ", " _ ", ":", " | "];

    for delim in &delimiters {
        if let Some(idx) = file_stem.find(delim) {
            let (left, right) = file_stem.split_at(idx);
            let right = &right[delim.len()..];
            return (
                Some(left.trim().to_string()),
                Some(right.trim().to_string()),
            );
        }
    }
    (None, Some(file_stem.trim().to_string()))
}
