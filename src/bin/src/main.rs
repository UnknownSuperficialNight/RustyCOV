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
        .arg(Arg::new("cov_address").short('c').long("cov-address-url").num_args(1).value_name("cov_address_url").help("Address of the COV website to be opened on launch"));

    // Conditionally add arguments
    #[cfg(feature = "jpeg-opt")]
    {
        cmd = cmd
            .arg(
                Arg::new("convert_png_to_jpg")
                    .short('j')
                    .long("convert-png-to-jpg")
                    .help("Convert PNG to JPG")
                    .long_help("If a PNG is selected, convert it to JPG to save space")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("jpeg_optimise")
                    .long("jpeg-optimise")
                    .help("Optimise JPEG images")
                    .action(ArgAction::SetTrue),
            );
    }

    #[cfg(feature = "png-opt")]
    {
        cmd = cmd.arg(
            Arg::new("png_opt")
                .long("png-opt")
                .help("Optimise PNG images")
                .action(ArgAction::SetTrue),
        );
    }

    let matches = cmd.get_matches();

    if let Some(raw) = matches.get_one::<String>("input_string") {
        if let Some(cov_address) = matches.get_one::<String>("cov_address") {
            match run(
                raw,
                Some(cov_address),
                matches.get_flag("convert_png_to_jpg"),
                matches.get_flag("jpeg_optimise"),
                matches.get_flag("png_opt"),
            ) {
                Ok(_) => {}
                Err(e) => eprintln!("Failed to run application: {}", e),
            }
        } else {
            match run(
                raw,
                None,
                matches.get_flag("convert_png_to_jpg"),
                matches.get_flag("jpeg_optimise"),
                matches.get_flag("png_opt"),
            ) {
                Ok(_) => {}
                Err(e) => eprintln!("Failed to run application: {}", e),
            }
        }
    } else {
        eprintln!("No input string provided.");
    }
}
