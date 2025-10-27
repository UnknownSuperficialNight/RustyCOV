# RustyCOV

A command-line tool for automatically finding and embedding album covers in audio files. This tool integrates with the COV website to discover covers, supports multiple file formats, and provides options for image optimisations.

## Showcase Album folder mode
[![Watch the video](https://img.youtube.com/vi/UN-AW4OSk3Y/maxresdefault.jpg)](https://youtu.be/UN-AW4OSk3Y)


## Features
- 🎵 Semi-Automated cover art retrieval from [covers.musichoarders.xyz](https://covers.musichoarders.xyz)
- 🖼️ Support for PNG/JPEG image conversion and optimisation
- 📁 Album folder mode for batch processing (writes cover to disk and removes embedded images from all songs in the folder)
- 📂 Recursive directory scanning for supported file formats
- 🔄 Embeds downloaded cover art into individual files by default
- 📦 Automatic dependency management (ffmpeg, covit)

## Supported File Formats (Not all tested yet)
- MP3
- M4A
- FLAC (Tested)
- AAC
- OPUS (Tested)
- OGG
- WMA
- WAV
- AIFF
- ALAC
- APE
- FLV
- WEBM

## Supported Image Formats
- PNG
- JPEG

## Installation
1. Ensure [Rust](https://rust-lang.org) is installed
2. Clone the repository
3. Build the project:

```bash
cargo build --release
```
or to build and run immediately
```bash
cargo run --release
```

### External Dependencies (Automatically Handled/Installed)
- [ffmpeg](https://ffmpeg.org) for fallback processing (Download implemented, but usage not yet implemented)
- [covit](https://covers.musichoarders.xyz) for cover discovery
