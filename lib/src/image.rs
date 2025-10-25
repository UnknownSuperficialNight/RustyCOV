#[cfg(feature = "jpeg-opt")]
use std::io::Cursor;

#[cfg(feature = "jpeg-opt")]
use lofty::picture::Picture;

#[cfg(feature = "jpeg-opt")]
pub fn convert_and_optimise_png_to_jpeg(
    cursor: &mut Cursor<Vec<u8>>,
    picture: &mut Picture,
) -> Result<(), Box<dyn std::error::Error>> {
    use image::{ImageFormat, ImageReader};

    cursor.set_position(0);

    // Decode PNG from memory
    let img = ImageReader::new(&mut *cursor)
        .with_guessed_format()?
        .decode()?;

    // Encode as JPEG into a new Vec<u8>
    let mut jpeg_bytes = Vec::new();
    img.write_to(&mut Cursor::new(&mut jpeg_bytes), ImageFormat::Jpeg)?;

    // Replace the original buffer
    *cursor.get_mut() = jpeg_bytes;
    cursor.set_position(0);

    *picture = Picture::from_reader(&mut *cursor)?;

    Ok(())
}

#[cfg(feature = "png-opt")]
pub fn optimise_png(
    cursor: &mut std::io::Cursor<Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error>> {
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
