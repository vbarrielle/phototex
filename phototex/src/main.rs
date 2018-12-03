use std::path::{Path, PathBuf};
use glob::glob;

fn find_images(images: &str, im_ext: &str) -> Vec<Vec<PathBuf>> {
    let images = Path::new(&images);
    // unwraping on pattern because a bad pattern is a programming error here
    let mut paths = Vec::new();
    for folder in glob(&images.join("*").to_string_lossy()).unwrap() {
        if let Ok(folder) = folder {
            let mut images = Vec::new();
            for image in glob(
                &folder.join(format!("*.{}", im_ext)).to_string_lossy()
            ).unwrap() {
                if let Ok(image) = image {
                    log::info!("Including image {:?}", image);
                    images.push(image);
                } else {
                    log::warn!("Ignoring image {:?}", image.unwrap_err());
                }
            }
            paths.push(images);
        } else {
            log::warn!("Ignoring folder {:?}", folder.unwrap_err());
        }
    }
    paths
}

fn main() {
    let matches = clap::App::new("phototex")
        .version("0.1")
        .author("Vincent Barrielle <vincent.barrielle@m4x.org>")
        .about("Generates latex files for photo albums")
        .arg(
            clap::Arg::with_name("images")
            .value_name("FOLDER")
            .help("Path to the images selection folders")
            .takes_value(true)
        )
        .arg(
            clap::Arg::with_name("im_ext")
            .long("--image_ext")
            .value_name("IMAGE_EXT")
            .help("Extension of images files. Defaults to 'jpg'")
            .takes_value(true)
        )
        .arg(clap::Arg::with_name("verbosity")
             .short("v")
             .multiple(true)
             .help("Increase message verbosity"))
        .get_matches();

    let verbosity = matches.occurrences_of("verbosity") as usize;

    stderrlog::new()
        .verbosity(verbosity + 1)
        .init()
        .unwrap();

    let images = matches
        .value_of("images")
        .expect("Path to images is mandatory");

    let im_ext = matches.value_of("im_ext").unwrap_or("jpg");

    log::info!("Using images path: {}", images);

    let im_paths = find_images(images, im_ext);
}
