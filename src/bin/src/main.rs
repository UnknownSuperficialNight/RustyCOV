#[cfg_attr(not(any(feature = "jpeg-opt", feature = "png-opt")), expect(unused_imports))]
use clap::{Arg, ArgAction, command};
use rusty_cov::run;

fn main() {
    #[cfg_attr(not(any(feature = "jpeg-opt", feature = "png-opt")), expect(unused_mut))]
    let mut cmd = command!()
        .arg(
            Arg::new("input_string")
                .short('i')
                .long("input")
                .num_args(1)
                .value_name("PATH")
                .help("Input directory or file to process")
                .long_help("Specify a directory to recursively process or a single file to process. Defaults to current directory."),
        )
        .arg(
            Arg::new("cov_url")
                .short('c')
                .long("cov-url")
                .num_args(1)
                .value_name("COV_ADDRESS_URL")
                .help("Address of the COV website to open on launch.")
                .long_help("Enter the URL of the COV website that you want to be opened when the application launches."),
            )
        .arg(
            Arg::new("album_mode")
                .short('a')
                .long("album-mode")
                .num_args(1)
                .value_name("COVER_NAME")
                .help("Process in album folder mode")
                .long_help("Write the selected image into the directory with the associated song and remove embedded images from other music files in the directory, resulting in each folder having a single album cover image."),
            );

    // Conditionally add arguments
    #[cfg(feature = "jpeg-opt")]
    {
        use clap::value_parser;

        cmd = cmd
            .arg(
                Arg::new("png_to_jpeg")
                    .long("png-to-jpeg")
                    .help("Convert PNG images to JPEG format")
                    .long_help("If a PNG is selected, convert it to JPG format to save space")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("jpeg_optimise")
                    .short('j')
                    .long("jpeg-optimise")
                    .help("Optimise JPEG images with specified quality (0-100, recommended: 80)")
                    .value_name("JPEG_QUALITY_NUMBER")
                    .value_parser(value_parser!(u8)),
            )
    }

    #[cfg(feature = "png-opt")]
    {
        cmd = cmd.arg(
            Arg::new("png_optimise")
                .short('p')
                .long("png-optimise")
                .help("Optimise PNG images")
                .long_help("Optimize PNG images to reduce file size")
                .action(ArgAction::SetTrue),
        );
    }

    let matches = cmd.get_matches();

    let input = match matches.get_one::<String>("input_string") {
        Some(s) => s.as_str(),
        None => ".",
    };
    let cov_address = matches.get_one::<String>("cov_url").map(|s| s.as_str());
    let cover_image_name = matches.get_one::<String>("album_mode").map(|s| s.as_str());

    match run(
        input,
        cov_address,
        matches.get_flag("png_to_jpeg"),
        matches.get_one::<u8>("jpeg_optimise").copied(),
        matches.get_flag("png_optimise"),
        cover_image_name,
    ) {
        Ok(_) => {}
        Err(e) => eprintln!("Failed to run application: {}", e),
    }
}
