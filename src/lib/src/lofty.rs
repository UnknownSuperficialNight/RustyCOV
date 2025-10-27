use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use lofty::config::{GlobalOptions, WriteOptions, apply_global_options};
use lofty::picture::{Picture, PictureType};
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::Tag;

#[cfg(feature = "png-opt")]
use crate::image::optimise_png;
#[cfg(feature = "jpeg-opt")]
use crate::image::{convert_png_to_jpeg, optimise_jpeg};

const ALLOCATION_LIMIT: usize = 1024 * 1024 * 1024;

/// Embeds a cover image into an audio file.
///
/// This function reads an audio file, downloads and processes an image from the given `image_url`,
/// and embeds it as a front cover in the audio file. Optionally converts PNG images to JPEG,
/// optimises JPEG images, and optimises PNG images if enabled.
///
/// # Arguments
///
/// * `audio_path` - Path to the audio file.
/// * `image_bytes` - The image data to embed in the audio file.
/// * `convert_png_to_jpg` - Whether to convert PNG images to JPEG before embedding.
/// * `jpeg_optimise` - Optimise the JPEG image using the specified quality (1-100) or None for no
///   optimisation.
/// * `png_opt` - Whether to optimise PNG images.
pub fn embed_cover_image<P: AsRef<Path>>(
    audio_path: P,
    image_bytes: Vec<u8>,
    convert_png_to_jpg: Arc<AtomicBool>,
    jpeg_optimise: Option<u8>,
    png_opt: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let global_options = GlobalOptions::new().allocation_limit(ALLOCATION_LIMIT);
    apply_global_options(global_options);

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

    // Process the image and get the processed bytes and Picture
    let (_, mut picture) = process_cover_image(image_bytes, &convert_png_to_jpg, jpeg_optimise, &png_opt)?;

    picture.set_pic_type(PictureType::CoverFront);

    // Remove any existing front cover, then add the new one
    tag.remove_picture_type(PictureType::CoverFront);
    tag.push_picture(picture);

    // Save the tag back to the file
    tag.save_to_path(audio_path, WriteOptions::new().respect_read_only(false))?;

    Ok(())
}

/// Processes the cover image based on the specified options.
///
/// This function handles converting and optimising PNG images to JPEG, as well as optimising JPEG
/// and PNG images, if `convert_png_to_jpg` or `png_opt` are set. It returns the processed image
/// bytes and a Picture object.
///
/// # Arguments
///
/// * `image_bytes` - The original image data in bytes.
/// * `convert_png_to_jpg` - Whether to convert PNG images to JPEG before processing.
/// * `jpeg_optimise` - Whether to optimise JPEG images.
/// * `jpeg_quality` - Optimise the JPEG image using the specified quality (1-100) or None for no
///   optimisation.
/// * `png_opt` - Whether to optimise PNG images.
pub fn process_cover_image(
    image_bytes: Vec<u8>,
    #[cfg_attr(not(feature = "jpeg-opt"), expect(unused_variables))] convert_png_to_jpg: &Arc<AtomicBool>,
    #[cfg_attr(not(feature = "jpeg-opt"), expect(unused_variables))] jpeg_optimise: Option<u8>,
    #[cfg_attr(not(feature = "png-opt"), expect(unused_variables))] png_opt: &Arc<AtomicBool>,
) -> Result<(Vec<u8>, Picture), Box<dyn std::error::Error>> {
    use std::io::Cursor;
    #[cfg_attr(not(any(feature = "jpeg-opt", feature = "png-opt")), expect(unused_imports))]
    use std::sync::atomic::Ordering;

    use lofty::picture::{MimeType, Picture};

    let mut cursor = Cursor::new(image_bytes);

    #[cfg_attr(not(any(feature = "jpeg-opt", feature = "png-opt")), expect(unused_mut))]
    let mut picture = Picture::from_reader(&mut cursor)?;

    match picture.mime_type() {
        Some(MimeType::Png) => {
            #[cfg(feature = "jpeg-opt")]
            if convert_png_to_jpg.load(Ordering::Relaxed) {
                convert_png_to_jpeg(&mut cursor, &mut picture, jpeg_optimise)?;
            }

            #[cfg(feature = "png-opt")]
            if picture.mime_type() == Some(&MimeType::Png) && png_opt.load(Ordering::Relaxed) {
                optimise_png(&mut cursor)?;
                picture = Picture::from_reader(&mut cursor)?;
            }
        }
        Some(MimeType::Jpeg) =>
        {
            #[cfg(feature = "jpeg-opt")]
            if let Some(jpeg_quality) = jpeg_optimise {
                optimise_jpeg(&mut cursor, jpeg_quality)?;
                picture = Picture::from_reader(&mut cursor)?;
            }
        }
        _ => {}
    }

    // Return the processed image bytes and the Picture
    Ok((cursor.into_inner(), picture))
}

/// Removes any embedded front cover image from an audio file.
///
/// This function reads the specified audio file, removes the primary front cover image if present,
/// and saves the changes back to the original file.
///
/// # Arguments
///
/// * `file_path` - Path to the audio file.
pub fn remove_embedded_art_from_file(file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut tagged_file = Probe::open(file_path)?.read()?;
    if let Some(tag) = tagged_file.primary_tag_mut() {
        while !tag.pictures().is_empty() {
            tag.remove_picture(0);
        }
        tag.save_to_path(file_path, WriteOptions::new().respect_read_only(false))?;
    }
    Ok(())
}
