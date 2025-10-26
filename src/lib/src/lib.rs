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

use serde_json::Value;

use crate::deps_download::download_and_extract_deps;
use crate::helpers::download_image;
use crate::lofty::{embed_cover_image, process_cover_image, remove_embedded_art_from_file};
use crate::structs::{CoverInfo, Picked, ReleaseInfo, RustyCov};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

const QUERTY_SOURCE: &str =
    "booth,amazonmusic,applemusic,musicbrainz,discogs,fanarttv,soundcloud,itunes,tidal";
const QUERY_COUNTRY: &str = "gb";

/// Runs the main logic of the application.
///
/// # Arguments
///
/// * `input_string` - Input directory or file to process.
/// * `cov_address` - Address of the COV website for launch.
/// * `convert_png_to_jpg` - Whether to convert PNG images to JPEG before embedding.
/// * `jpeg_optimise` - Whether to optimize JPEG images.
/// * `png_opt` - Whether to optimize PNG images.
/// * `album_folder_mode` - Whether to use the album folder mode.
///
/// # Returns
///
/// Result indicating success or an error if any step fails.
pub fn run(
    input_string: &str,
    cov_address: Option<&str>,
    convert_png_to_jpg: bool,
    jpeg_optimise: bool,
    jpeg_quality: Option<u8>,
    png_opt: bool,
    album_folder_mode: Option<&str>,
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

    match &mut rusty_cov_global.files {
        Some(files_by_dir) if !files_by_dir.is_empty() => {
            if let Some(album_name) = album_folder_mode {
                // --- Album Folder Mode ---
                let mut completed = 0usize;
                for (dir, files) in files_by_dir.iter() {
                    // Check if art already exists (either .jpg or .png)
                    let jpg_path = dir.join(format!("{}.jpg", album_name));
                    let png_path = dir.join(format!("{}.png", album_name));
                    if jpg_path.exists() || png_path.exists() {
                        println!("Album art already exists in {:?}, skipping.", dir);
                        continue;
                    }

                    // Try each file in the folder until run_covit succeeds
                    let mut picked_opt = None;
                    for file in files {
                        if let Some(picked) = run_covit(
                            rusty_cov_global.deps.as_ref().unwrap().covit.as_str(),
                            rusty_cov_global.cov_address.unwrap(),
                            file,
                        ) {
                            picked_opt = Some(picked);
                            break;
                        }
                    }

                    if let Some(picked) = picked_opt {
                        println!(
                            "Folder: {:?}\nArtist: {}\nTitle: {}\nDate: {}\nCover Type: {}\nImage Size: {} bytes\nDimensions: {}x{}\nBig Cover URL: {}\n",
                            dir,
                            picked.release_info.artist,
                            picked.release_info.title,
                            picked.release_info.date,
                            picked.cover_info.format,
                            picked.cover_info.size,
                            picked.cover_info.width,
                            picked.cover_info.height,
                            picked.big_cover_url
                        );

                        // Download the image
                        let image_bytes = download_image(&picked.big_cover_url)?;

                        let (processed_bytes, _) = process_cover_image(
                            image_bytes,
                            &convert_png_to_jpg,
                            &jpeg_optimise,
                            jpeg_quality,
                            &png_opt,
                        )?;

                        let art_path =
                            dir.join(format!("{}.{}", album_name, picked.cover_info.format));
                        std::fs::write(&art_path, &processed_bytes)?;
                        println!("Saved album art to {:?}", art_path);

                        // Remove embedded art from all files in this folder
                        for file in files {
                            if let Err(e) = remove_embedded_art_from_file(file) {
                                eprintln!("Failed to remove embedded art from {:?}: {}", file, e);
                            } else {
                                println!("Removed embedded art from {:?}", file);
                            }
                        }
                        completed += 1;
                    } else {
                        println!("No cover info found for folder {:?}", dir);
                    }
                }
                println!("Summary: {} folder(s) finished.", completed);
            } else {
                // --- Per-File Mode ---
                let mut handles: HashMap<usize, std::thread::JoinHandle<()>> = HashMap::new();
                let mut job_id = 0usize;
                for (_dir, files) in files_by_dir.iter_mut() {
                    for path in files.drain(..) {
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
                                // Download the image using ureq
                                let image_bytes = download_image(&picked.big_cover_url)
                                    .expect("Failed to Download Image");

                                if let Err(e) = embed_cover_image(
                                    path,
                                    image_bytes,
                                    convert_png_to_jpg,
                                    jpeg_optimise,
                                    jpeg_quality,
                                    png_opt,
                                ) {
                                    eprintln!("Failed to embed cover: {}", e);
                                }
                            });
                            handles.insert(job_id, handle);
                            job_id += 1;
                        } else {
                            println!("No cover info found for {:?}", path);
                        }
                    }
                }

                let mut completed = 0usize;
                for (job_id, handle) in handles {
                    match handle.join() {
                        Ok(_) => completed += 1,
                        Err(panic) => eprintln!("Job {} panicked: {:?}", job_id, panic),
                    }
                }
                println!("Summary: {} job(s) finished.", completed);
            }
        }
        _ => eprintln!("No files were found or the input was invalid."),
    }
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
        .arg("--query-sources")
        .arg(QUERTY_SOURCE)
        .arg("--query-country")
        .arg(QUERY_COUNTRY)
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
        if let Some(json) = line.strip_prefix("Picked: ") {
            // Attempt to parse into serde_json::Value first
            if let Ok(value) = serde_json::from_str::<Value>(json) {
                // Now we can extract fields and create a Picked instance
                let picked = Picked {
                    big_cover_url: value
                        .get("bigCoverUrl")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    release_info: ReleaseInfo {
                        title: value
                            .get("releaseInfo")
                            .and_then(|v| v.get("title").and_then(Value::as_str))
                            .unwrap_or("Unknown Title")
                            .to_string(),
                        artist: value
                            .get("releaseInfo")
                            .and_then(|v| v.get("artist").and_then(Value::as_str))
                            .unwrap_or("Unknown Artist")
                            .to_string(),
                        date: value
                            .get("releaseInfo")
                            .and_then(|v| v.get("date").and_then(Value::as_str))
                            .unwrap_or("Unknown Date")
                            .to_string(),
                        tracks: value
                            .get("releaseInfo")
                            .and_then(|v| v.get("tracks").and_then(Value::as_u64))
                            .map(|v| v as u32),
                    },
                    cover_info: CoverInfo {
                        format: value
                            .get("coverInfo")
                            .and_then(|v| v.get("format").and_then(Value::as_str))
                            .unwrap_or("Unknown Format")
                            .to_string(),
                        height: value
                            .get("coverInfo")
                            .and_then(|v| v.get("height").and_then(Value::as_u64))
                            .unwrap_or(0) as u32,
                        width: value
                            .get("coverInfo")
                            .and_then(|v| v.get("width").and_then(Value::as_u64))
                            .unwrap_or(0) as u32,
                        size: value
                            .get("coverInfo")
                            .and_then(|v| v.get("size").and_then(Value::as_u64))
                            .unwrap_or(0),
                    },
                };

                return Some(picked);
            }
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
    cmd.arg("--address").arg(address).arg("--query-album").arg(title);

    if let Some(ref artist) = artist_opt &&
        !artist.is_empty()
    {
        cmd.arg("--query-artist").arg(artist);
    }

    cmd.arg("--remote-agent")
        .arg(format!("{} - {}", PROGRAM_NAME, VERSION))
        .arg("--query-sources")
        .arg(QUERTY_SOURCE)
        .arg("--query-country")
        .arg(QUERY_COUNTRY);

    cmd.output().ok()
}

/// Parses file name to extract artist and title.
fn parse_file_name(file_stem: &str) -> (Option<String>, Option<String>) {
    let delimiters = [" - ", " – ", " — ", " _ ", ":", " | "];

    for delim in &delimiters {
        if let Some(idx) = file_stem.find(delim) {
            let (left, right) = file_stem.split_at(idx);
            let right = &right[delim.len()..];
            return (Some(left.trim().to_string()), Some(right.trim().to_string()));
        }
    }
    (None, Some(file_stem.trim().to_string()))
}
