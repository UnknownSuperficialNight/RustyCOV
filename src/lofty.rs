use lofty::config::{GlobalOptions, WriteOptions, apply_global_options};
use lofty::picture::{Picture, PictureType};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::Tag;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use ureq;

const ALLOCATION_LIMIT: usize = 1024 * 1024 * 1024;

pub fn embed_cover_image<P: AsRef<Path>>(
    audio_path: P,
    image_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let global_options = GlobalOptions::new().allocation_limit(ALLOCATION_LIMIT);
    apply_global_options(global_options);

    // Download the image using ureq
    let response = ureq::get(image_url).call()?;
    if response.status() != 200 {
        return Err(format!("Failed to download image: HTTP {}", response.status()).into());
    }
    let mut image_data = Vec::new();
    let (_, body) = response.into_parts();

    body.into_reader().read_to_end(&mut image_data)?;

    // Open the audio file with lofty
    let mut tagged_file = Probe::open(&audio_path)?.read()?;

    // Get or create the tag
    let tag = match tagged_file.primary_tag_mut() {
        Some(primary_tag) => primary_tag,
        None => {
            if let Some(first_tag) = tagged_file.first_tag_mut() {
                first_tag
            } else {
                let tag_type = tagged_file.primary_tag_type();
                tagged_file.insert_tag(Tag::new(tag_type));
                tagged_file.primary_tag_mut().unwrap()
            }
        }
    };

    // Create a Picture from the in-memory image data
    let mut cursor = Cursor::new(image_data);
    let mut picture = Picture::from_reader(&mut cursor)?;
    picture.set_pic_type(PictureType::CoverFront);

    // Remove any existing front cover, then add the new one
    tag.remove_picture_type(PictureType::CoverFront);
    tag.push_picture(picture);

    // Save the tag back to the file
    tag.save_to_path(audio_path, WriteOptions::new().respect_read_only(false))?;

    Ok(())
}
