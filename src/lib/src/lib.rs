pub mod deps_download;
pub mod helpers;
#[doc(hidden)]
pub mod image;

pub mod lofty;
pub mod structs;

use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::{
    deps_download::download_and_extract_deps,
    lofty::embed_cover_image,
    structs::{Picked, RustyCov},
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

/// Runs the main logic of the application.
///
/// # Arguments
///
/// * `input_string` - Input directory or file to process.
/// * `cov_address` - Address of the COV website for launch.
/// * `convert_png_to_jpg` - Whether to convert PNG images to JPEG before embedding.
/// * `jpeg_optimise` - Whether to optimize JPEG images.
/// * `png_opt` - Whether to optimize PNG images.
///
/// # Returns
///
/// Result indicating success or an error if any step fails.
pub fn run(
    input_string: &str,
    cov_address: Option<&str>,
    convert_png_to_jpg: bool,
    jpeg_optimise: bool,
    png_opt: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rusty_cov_global = RustyCov::default();

    // Populate files from input
    rusty_cov_global.populate_from_input(input_string);

    if let Some(cov_address) = cov_address {
        rusty_cov_global.cov_address = Some(cov_address);
    }

    // Download dependencies
    match download_and_extract_deps() {
        Ok(deps) => {
            rusty_cov_global.deps = Some(deps);
        }
        Err(e) => {
            eprintln!("Failed to download dependencies: {e}");
            return Err(e);
        }
    }

    // Create atomic bools for features
    let convert_png_to_jpg = Arc::new(AtomicBool::new(convert_png_to_jpg));
    let jpeg_optimise = Arc::new(AtomicBool::new(jpeg_optimise));
    let png_opt = Arc::new(AtomicBool::new(png_opt));

    // If no files were found, exit.
    if rusty_cov_global.files.is_none() {
        eprintln!("No supported audio/video files were found exiting.");
        return Ok(());
    }

    // Store handles so we can wait for them later
    let mut handles: HashMap<usize, std::thread::JoinHandle<()>> = HashMap::new();

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

                    let handle = std::thread::spawn(move || {
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
    Ok(())
}

/// Run covit and return the picked file.
pub fn run_covit(covit_path: &str, address: &str, input: &std::path::PathBuf) -> Option<Picked> {
    use std::process::Command;

    // First attempt: run covit normally
    let output = Command::new(covit_path)
        .arg("--address")
        .arg(address)
        .arg("--input")
        .arg(input)
        .arg("--remote-agent")
        .arg(format!("{} - {}", PROGRAM_NAME, VERSION))
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

/// Parses covit output to extract Picked information.
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

/// Runs covit with --query-artist and --query-album.
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
        .arg("--remote-agent")
        .arg(format!("{} - {}", PROGRAM_NAME, VERSION))
        .arg(title);

    if let Some(ref artist) = artist_opt
        && !artist.is_empty()
    {
        cmd.arg("--query-artist").arg(artist);
    }

    cmd.output().ok()
}

/// Parses file name to extract artist and title.
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
