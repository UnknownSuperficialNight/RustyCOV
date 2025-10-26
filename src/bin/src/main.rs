use clap::{Arg, ArgAction, command};
use rusty_cov::run;

fn main() {
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
        .arg(
                Arg::new("cov_address")
                    .short('c')
                    .long("cov-address-url")
                    .num_args(1)
                    .value_name("COV_ADDRESS_URL")
                    .help("Address of the COV website to open on launch.")
                    .long_help("Enter the URL of the COV website that you want to be opened when the application launches."),
            )
        .arg(
                Arg::new("album_folder_mode")
                    .short('a')
                    .long("album-folder-mode")
                    .num_args(1)
                    .value_name("COVER_IMAGE_NAME")
                    .help("Write images to folder and remove embedded images from all files within the directory.")
                    .long_help("This mode writes the selected image into the directory with the associated song then removes embedded images from other music files in the associated directory, resulting in each folder having a single album cover image."),
            );

    // Conditionally add arguments
    #[cfg(feature = "jpeg-opt")]
    {
        use clap::value_parser;

        cmd = cmd
            .arg(
                Arg::new("convert_png_to_jpg")
                    .short('j')
                    .long("convert-png-to-jpg")
                    .help("Convert PNG to JPG")
                    .long_help("If a PNG is selected, convert it to JPG format to save space")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("jpeg_optimise")
                    .long("jpeg-optimise")
                    .help("Optimise JPEG images")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("jpeg_quality")
                    .long("jpeg-quality")
                    .help("Set the quality to encode the jpeg as as can be between 0 and 100 (default: 80)")
                    .value_name("JPEG_QUALITY_NUMBER")
                    .value_parser(value_parser!(u8)),
            );
    }

    #[cfg(feature = "png-opt")]
    {
        cmd = cmd.arg(
            Arg::new("png_optimise")
                .long("png-optimise")
                .help("Optimise PNG images")
                .action(ArgAction::SetTrue),
        );
    }

    let matches = cmd.get_matches();

    if let Some(raw) = matches.get_one::<String>("input_string") {
        let cov_address = matches.get_one::<String>("cov_address").map(|s| s.as_str());
        let cover_image_name = matches.get_one::<String>("album_folder_mode").map(|s| s.as_str());

        match run(
            raw,
            cov_address,
            matches.get_flag("convert_png_to_jpg"),
            matches.get_flag("jpeg_optimise"),
            matches.get_one::<u8>("jpeg_quality").copied(),
            matches.get_flag("png_optimise"),
            cover_image_name,
        ) {
            Ok(_) => {}
            Err(e) => eprintln!("Failed to run application: {}", e),
        }
    } else {
        eprintln!("No input string provided.");
    }
}
