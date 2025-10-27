use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use walkdir::WalkDir;

use crate::deps_download::DependencyPaths;
use crate::helpers::extract_first_number;

/// Supported audio/video file extensions.
#[derive(Debug, PartialEq, Eq)]
pub enum FileFormat {
    Mp3,
    M4a,
    Flac,
    Aac,
    Opus,
    Ogg,
    Wma,
    Wav,
    Aiff,
    Alac,
    Ape,
    Flv,
    Webm,
    Unknown,
}

impl FileFormat {
    /// Return the enum variant that matches the file’s extension (case‑insensitive).
    fn from_path<P: AsRef<Path>>(path: P) -> Self {
        match path
            .as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("mp3") => FileFormat::Mp3,
            Some("m4a") => FileFormat::M4a,
            Some("flac") => FileFormat::Flac,
            Some("aac") => FileFormat::Aac,
            Some("opus") => FileFormat::Opus,
            Some("ogg") => FileFormat::Ogg,
            Some("wma") => FileFormat::Wma,
            Some("wav") => FileFormat::Wav,
            Some("aiff") => FileFormat::Aiff,
            Some("alac") => FileFormat::Alac,
            Some("ape") => FileFormat::Ape,
            Some("flv") => FileFormat::Flv,
            Some("webm") => FileFormat::Webm,
            _ => FileFormat::Unknown,
        }
    }

    /// Helper to know whether the variant is a real format.
    fn is_known(&self) -> bool {
        *self != FileFormat::Unknown
    }
}

/// Holds the list of files (with detected format) for the supplied input.
pub struct RustyCov<'a> {
    /// `None` → no input processed yet; `Some(map)` → files grouped by parent directory.
    pub files: Option<HashMap<PathBuf, Vec<PathBuf>>>,
    pub deps: Option<DependencyPaths>,
    pub cov_address: Option<&'a str>,
}

impl<'a> Default for RustyCov<'a> {
    fn default() -> Self {
        Self { files: None, deps: None, cov_address: Some("https://covers.musichoarders.xyz") }
    }
}

impl<'a> RustyCov<'a> {
    /// Populate `files` from a path that may be a file or a directory.
    /// Only entries whose extension maps to a known `FileFormat` are kept.
    pub fn populate_from_input<S: Into<String>>(&mut self, input: S) {
        let path_str = input.into();
        let path = PathBuf::from(&path_str);

        let mut files_by_dir: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        if path.is_dir() {
            // Walk the directory recursively, keeping only known formats.
            for entry in WalkDir::new(&path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                add_file_to_map(&mut files_by_dir, entry.path());
            }
        } else if path.is_file() {
            // Single file case – keep it only if it matches a known format.
            add_file_to_map(&mut files_by_dir, &path);
        } else {
            eprintln!("❌ Path '{}' does not exist.", path_str);
            self.files = None;
            return;
        }

        // Sort files in each directory according numeric ordering rule
        for files in files_by_dir.values_mut() {
            files.sort_by(|a, b| {
                let a_name = a.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let b_name = b.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                // Extract first digit substring and its length from a_name
                let a_num_opt = extract_first_number(a_name);
                let b_num_opt = extract_first_number(b_name);

                match (a_num_opt, b_num_opt) {
                    (Some((a_num, a_len)), Some((b_num, b_len))) => {
                        match a_num.cmp(&b_num) {
                            std::cmp::Ordering::Equal => b_len.cmp(&a_len), /* longer digit */
                            // substring (leading
                            // zeros) first
                            other => other,
                        }
                    }
                    (Some(_), None) => std::cmp::Ordering::Less, // numbers come before no numbers
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a_name.cmp(b_name), // fallback lex order
                }
            });
        }

        // If we gathered at least one supported file, store it; otherwise keep None.
        if !files_by_dir.is_empty() {
            self.files = Some(files_by_dir);
        }
    }
}

/// Adds a file to the map grouped by its parent directory if the file's format is known.
///
/// 1. Determines the file's format using `FileFormat::from_path`
/// 2. Checks if the format is known via `is_known()`
/// 3. If both conditions are met, adds the file to the corresponding directory entry in the
///    HashMap. Files without parent directories (e.g., root path) are skipped.
fn add_file_to_map(files_by_dir: &mut HashMap<PathBuf, Vec<PathBuf>>, file_path: &Path) {
    let fmt = FileFormat::from_path(file_path);
    if fmt.is_known() &&
        let Some(parent) = file_path.parent()
    {
        files_by_dir.entry(parent.to_path_buf()).or_default().push(file_path.to_path_buf());
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Picked {
    pub big_cover_url: String,
    pub release_info: ReleaseInfo,
    pub cover_info: CoverInfo,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub title: String,
    pub artist: String,
    pub date: String,
    pub tracks: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CoverInfo {
    pub format: String,
    pub height: u32,
    pub width: u32,
    pub size: u64,
}
