use std::path::{Path, PathBuf};
use glob::glob;
use image::{ImageDecoder, ImageResult};

struct ImageInfo {
    path: PathBuf,
    dimensions: (u32, u32),
}

fn image_dimensions(path: &Path) -> ImageResult<(u32, u32)> {
    let fin = std::fs::File::open(path)?;
    let fin = std::io::BufReader::new(fin);

    let ext = path.extension()
        .and_then(|s| s.to_str())
        .map_or("".to_string(), |s| s.to_ascii_lowercase());

     match &ext[..] {
        //#[cfg(feature = "jpeg")]
        "jpg" | "jpeg" => image::jpeg::JPEGDecoder::new(fin).dimensions(),
        //#[cfg(feature = "png_codec")]
        "png" => image::png::PNGDecoder::new(fin).dimensions(),
        //#[cfg(feature = "gif_codec")]
        "gif" =>  image::gif::Decoder::new(fin).dimensions(),
        //#[cfg(feature = "webp")]
        "webp" => image::webp::WebpDecoder::new(fin).dimensions(),
        //#[cfg(feature = "tiff")]
        "tif" | "tiff" => image::tiff::TIFFDecoder::new(fin)?.dimensions(),
        //#[cfg(feature = "tga")]
        "tga" => image::tga::TGADecoder::new(fin).dimensions(),
        //#[cfg(feature = "bmp")]
        "bmp" => image::bmp::BMPDecoder::new(fin).dimensions(),
        //#[cfg(feature = "ico")]
        "ico" => image::ico::ICODecoder::new(fin)?.dimensions(),
        //#[cfg(feature = "hdr")]
        "hdr" => image::hdr::HDRAdapter::new(fin)?.dimensions(),
        //#[cfg(feature = "pnm")]
        "pbm" | "pam" | "ppm" | "pgm" => {
            image::pnm::PNMDecoder::new(fin)?.dimensions()
        },
        format => {
            Err(image::ImageError::UnsupportedError(format!(
                "Image format image/{:?} is not supported.",
                format
            )))
        }
    }
}

fn find_images(images: &str, im_ext: &str) -> Vec<Vec<ImageInfo>> {
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
                    let image_dims = image_dimensions(&image);
                    if let Ok(image_dims) = image_dims {
                        log::info!(
                            "Including image {:?} ({}x{})",
                            image,
                            image_dims.0,
                            image_dims.1,
                        );
                        images.push(ImageInfo {
                            path: image,
                            dimensions: image_dims,
                        });
                    } else {
                        log::warn!("Could not open image {:?}", image);
                    }
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

fn replace(io_string: &mut String, pat: &str, replace_with: &str) -> Result<(), ()> {
    let pos = io_string.find(pat).ok_or(())?;
    io_string.replace_range(pos..(pos + pat.len()), replace_with);
    Ok(())
}

struct BookInfo {
    title: String,
}

fn write_toplevel(out_folder: &str, book_info: &BookInfo) -> std::io::Result<()> {
    use std::io::Write;
    let mut toplevel_text = include_str!("../data/toplevel.tex").to_string();
    replace(&mut toplevel_text, "PHOTOTEX_TITLE", &book_info.title);


    let toplevel_file = Path::new(&out_folder).join("photobook.tex");
    let f = std::fs::File::create(&toplevel_file)?;
    let mut writer = std::io::BufWriter::new(f);
    write!(writer, "{}", toplevel_text)?;
    Ok(())
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
            clap::Arg::with_name("out_folder")
            .short("-o")
            .value_name("OUT_FOLDER")
            .help("Path where the latex should be written. Defaults to .")
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
        .unwrap_or_else(|| {
            println!("{}", matches.usage());
            std::process::exit(1);
        });

    let out_folder = matches.value_of("out_folder").unwrap_or(".");

    let im_ext = matches.value_of("im_ext").unwrap_or("jpg");

    log::info!("Using images path: {}", images);

    let im_infos = find_images(images, im_ext);

    let book_info = BookInfo { title: "Titre".to_string() };
    write_toplevel(out_folder, &book_info);
}
