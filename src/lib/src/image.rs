#[cfg(feature = "jpeg-opt")]
use std::io::Cursor;

#[cfg(feature = "jpeg-opt")]
use image::ImageReader;
#[cfg(feature = "jpeg-opt")]
use lofty::picture::Picture;

/// Converts a PNG image to JPEG format.
///
/// This function reads a PNG image from the provided cursor, converts it to JPEG,
/// and replaces the original buffer with the new JPEG data. If `jpeg_optimise` is set,
/// the JPEG will also be optimised. The given `Picture`
/// object is updated with the new JPEG image data.
///
/// # Arguments
///
/// * `cursor` - A mutable cursor containing the PNG image data.
/// * `picture` - A mutable reference to a `Picture` object to update with the JPEG image.
/// * `jpeg_optimise` - Optimise the JPEG image using the specified quality (1-100) or None for no
///   optimisation.
#[cfg(feature = "jpeg-opt")]
pub(crate) fn convert_png_to_jpeg(
    cursor: &mut std::io::Cursor<Vec<u8>>,
    picture: &mut Picture,
    jpeg_optimise: Option<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    use image::ImageReader;

    cursor.set_position(0);

    // Decode PNG from memory
    let img = ImageReader::new(&mut *cursor).with_guessed_format()?.decode()?;

    // Encode PNG image as JPEG with recommended quality (80) into a new Vec<u8>
    // We just convert here, optimisation will be done in optimise_jpeg
    let mut jpeg_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut jpeg_bytes), image::ImageFormat::Jpeg)?;

    // Replace the original buffer with the JPEG data
    *cursor.get_mut() = jpeg_bytes;
    cursor.set_position(0);

    // Now call optimise_jpeg if requested, else just update picture
    if let Some(jpeg_quality) = jpeg_optimise {
        optimise_jpeg(cursor, jpeg_quality)?;
    }

    *picture = Picture::from_reader(&mut *cursor)?;

    Ok(())
}

/// Optimises a JPEG image in memory.
///
/// This function reads the JPEG data from the provided cursor, optimises it using the image crate,
/// and replaces the original buffer with the optimised data.
///
/// # Arguments
///
/// * `cursor` - A mutable cursor containing the JPEG image data.
/// * `quality` - The quality of the output JPEG image (1-100).
#[cfg(feature = "jpeg-opt")]
pub(crate) fn optimise_jpeg(cursor: &mut std::io::Cursor<Vec<u8>>, quality: u8) -> Result<(), Box<dyn std::error::Error>> {
    use image::codecs::jpeg::JpegEncoder;

    cursor.set_position(0);

    // Decode JPEG from memory
    let img = ImageReader::new(&mut *cursor).with_guessed_format()?.decode()?;

    // Encode as JPEG with specified quality into a new Vec<u8>
    let mut jpeg_bytes = Vec::new();
    {
        let mut encoder = JpegEncoder::new_with_quality(&mut jpeg_bytes, quality);
        encoder.encode_image(&img)?;
    }

    // Replace the original buffer
    *cursor.get_mut() = jpeg_bytes;
    cursor.set_position(0);

    Ok(())
}

/// Optimises a PNG image in memory.
///
/// This function reads the PNG data from the provided cursor, optimises it using oxipng,
/// and replaces the original buffer with the optimised data.
///
/// # Arguments
///
/// * `cursor` - A mutable cursor containing the PNG image data.
#[cfg(feature = "png-opt")]
pub(crate) fn optimise_png(cursor: &mut std::io::Cursor<Vec<u8>>) -> Result<(), Box<dyn std::error::Error>> {
    use oxipng::{Options as OxipngOptions, StripChunks, optimize_from_memory};

    // Get the PNG data from the cursor
    let data = cursor.get_ref();

    // Set up oxipng options
    let mut options = OxipngOptions::max_compression();
    options.strip = StripChunks::Safe;
    options.optimize_alpha = true;

    // Optimise the PNG data in memory
    let optimised_data = optimize_from_memory(data, &options)?;

    // Replace the cursor's buffer with the optimised data
    *cursor.get_mut() = optimised_data;
    cursor.set_position(0);

    Ok(())
}
