use std::path::{Path, PathBuf};
use glob::glob;
use image::{ImageDecoder, ImageResult};
use itertools::Itertools;
use std::io::Write;

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

enum PageKind {
    TwoLandscapes,
    OnePortrait,
}

struct PageInfo {
    path: PathBuf,
    kind: PageKind,
}

fn write_toplevel(
    out_folder: &str,
    book_info: &BookInfo,
    page_infos: &[PageInfo],
) -> std::io::Result<()> {
    let mut toplevel_text = include_str!("../data/toplevel.tex").to_string();
    replace(&mut toplevel_text, "PHOTOTEX_TITLE", &book_info.title).unwrap();
    let mut page_includes = String::new();
    for page in page_infos {
        if let Some(path) = page.path.to_str() {
            page_includes.push_str(&format!("\\input{{{}}}\n", path));
        } else {
            log::error!("could not include page {:?}", page.path);
        }
    }
    replace(
        &mut toplevel_text, "PHOTOTEX_PAGES_INCLUDE_PLACEHOLDER", &page_includes
    ).unwrap();


    let top_file_name = "photobook.tex";
    let toplevel_file = Path::new(&out_folder).join(top_file_name);
    let f = std::fs::File::create(&toplevel_file)?;
    let mut writer = std::io::BufWriter::new(f);
    write!(writer, "{}", toplevel_text)?;

    let makefile = Path::new(&out_folder).join("Makefile");
    let mut makefile_text = include_str!("../data/Makefile").to_string();
    replace(
        &mut makefile_text, "PHOTOTEX_TOPLEVEL_FILE_NAME", top_file_name
    ).unwrap();
    let f = std::fs::File::create(&makefile)?;
    let mut writer = std::io::BufWriter::new(f);
    write!(writer, "{}", makefile_text)?;
    Ok(())
}

fn write_pages(
    out_folder: &str,
    images: &[Vec<ImageInfo>]
) -> std::io::Result<Vec<PageInfo>> {
    let nb_images = images.iter().map(|v| v.len()).sum();
    let mut page_infos = Vec::with_capacity(nb_images);
    for im_group in images {
        for (im0, im1) in im_group
            .iter()
            .filter(|im| im.dimensions.0 >= im.dimensions.1)
            .tuples() {
            let page_id = page_infos.len();
            let page_path = Path::new(&out_folder)
                .join(format!("page{:03}", page_id));
            std::fs::create_dir_all(&page_path)?;
            let page_path = page_path.join("page.tex");
            let f = std::fs::File::create(&page_path)?;
            let mut writer = std::io::BufWriter::new(f);
            let mut page_text = include_str!("../data/page_2_landscapes.tex")
                .to_string();
            if let Some(im0_path) = im0.path.to_str() {
                replace(
                    &mut page_text, "PHOTOTEX_FIRST_IMAGE_PATH", im0_path,
                ).unwrap();
            } else {
                log::error!(
                    "could not include image path {:?} in {:?}: utf-8 failed",
                    im0.path, page_path,
                );
            }
            if let Some(im1_path) = im1.path.to_str() {
                replace(
                    &mut page_text, "PHOTOTEX_SECOND_IMAGE_PATH", im1_path,
                ).unwrap();
            } else {
                log::error!(
                    "could not include image path {:?} in {:?}: utf-8 failed",
                    im1.path, page_path,
                );
            }
            replace(&mut page_text, "PHOTOTEX_FIRST_LEGEND", "%").unwrap();
            replace(&mut page_text, "PHOTOTEX_SECOND_LEGEND", "%").unwrap();
            write!(writer, "{}", page_text)?;

            page_infos.push(PageInfo {
                path: page_path,
                kind: PageKind::TwoLandscapes,
            });
        }
    }
    Ok(page_infos)
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

    if let Ok(page_infos) = write_pages(out_folder, &im_infos) {
        let book_info = BookInfo { title: "Titre".to_string() };
        if let Err(e) = write_toplevel(out_folder, &book_info, &page_infos) {
            log::error!("Error writing toplevel: {:?}", e);
        }
    } else {
        log::error!("Could not write pages!");
    }
}
