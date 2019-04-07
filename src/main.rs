use glob::glob;
use image::{ImageDecoder, ImageOutputFormat, ImageResult};
use itertools::Itertools;
use rayon::prelude::*;
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};

use phototex::specs::FolderSpec;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum LayoutReq {
    OnePortrait,
    Nothing,
}

#[derive(Debug)]
struct SourceImageInfo {
    path: PathBuf,
    dimensions: (u32, u32),
    orientation: Orientation,
    user_req: LayoutReq,
}

#[derive(Debug)]
struct ImageInfo {
    path: PathBuf,
    resize_dims: (u32, u32),
    rotated_dims: (u32, u32),
    user_req: LayoutReq,
}

#[derive(Copy, Clone, Debug)]
enum Orientation {
    // Rotations are clockwise to match image crate
    Rotate90,
    Rotate270,
    Rotate180,
    Keep,
    Unknown,
    Flipped,
}

fn image_exif_orientation(path: &Path) -> Orientation {
    let thumbnail = false;
    let mut fin = std::fs::File::open(path).map(std::io::BufReader::new).ok();
    let reader = fin
        .as_mut()
        .ok_or(exif::Error::BlankValue("dummy"))
        .and_then(exif::Reader::new)
        .ok();
    let orientation = reader
        .as_ref()
        .and_then(|r| r.get_field(exif::Tag::Orientation, thumbnail));
    if let Some(orientation) = orientation {
        match &orientation.value {
            exif::Value::Short(vals) => {
                if vals.len() == 1 {
                    match vals[0] {
                        1 => Orientation::Keep,
                        3 => Orientation::Rotate180,
                        6 => Orientation::Rotate90,
                        8 => Orientation::Rotate270,
                        _ => Orientation::Flipped,
                    }
                } else {
                    Orientation::Unknown
                }
            }
            _ => {
                log::info!("Unknown orientation for {:?}", path);
                Orientation::Unknown
            }
        }
    } else {
        log::info!("Unknown orientation for {:?}", path);
        Orientation::Unknown
    }
}

fn image_dimensions(path: &Path) -> ImageResult<(u32, u32)> {
    let fin = std::fs::File::open(path)?;
    let fin = std::io::BufReader::new(fin);

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map_or("".to_string(), |s| s.to_ascii_lowercase());

    match &ext[..] {
        //#[cfg(feature = "jpeg")]
        "jpg" | "jpeg" => Ok(image::jpeg::JPEGDecoder::new(fin)?.dimensions()),
        //#[cfg(feature = "png_codec")]
        "png" => Ok(image::png::PNGDecoder::new(fin)?.dimensions()),
        //#[cfg(feature = "gif_codec")]
        "gif" => Ok(image::gif::Decoder::new(fin)?.dimensions()),
        //#[cfg(feature = "webp")]
        "webp" => Ok(image::webp::WebpDecoder::new(fin)?.dimensions()),
        //#[cfg(feature = "tiff")]
        "tif" | "tiff" => Ok(image::tiff::TIFFDecoder::new(fin)?.dimensions()),
        //#[cfg(feature = "tga")]
        "tga" => Ok(image::tga::TGADecoder::new(fin)?.dimensions()),
        //#[cfg(feature = "bmp")]
        "bmp" => Ok(image::bmp::BMPDecoder::new(fin)?.dimensions()),
        //#[cfg(feature = "ico")]
        "ico" => Ok(image::ico::ICODecoder::new(fin)?.dimensions()),
        //#[cfg(feature = "hdr")]
        "hdr" => Ok(image::hdr::HDRAdapter::new(fin)?.dimensions()),
        //#[cfg(feature = "pnm")]
        "pbm" | "pam" | "ppm" | "pgm" => {
            Ok(image::pnm::PNMDecoder::new(fin)?.dimensions())
        }
        format => Err(image::ImageError::UnsupportedError(format!(
            "Image format image/{:?} is not supported.",
            format
        ))),
    }
    .map(|(w, h)| (w as u32, h as u32)) // TODO panic on overflow
}

fn compute_good_dimensions(
    in_dims: (u32, u32),
    (page_width, page_height): (f32, f32), // in mm
    dpm: f32,                              // dots per mm
) -> (u32, u32) {
    // we need to have 300 dpi ie 12 dots per mm
    let (target_width, target_height) = (page_width * dpm, page_height * dpm);
    let (in_w, in_h) = (in_dims.0 as f32, in_dims.1 as f32);
    let width_factor = target_width / in_w;
    let height_factor = target_height / in_h;
    let factor = width_factor.min(height_factor);
    if factor > 1. {
        log::warn!(
            "image of resolution {}x{} is too small for dpm {}",
            in_dims.0,
            in_dims.1,
            dpm,
        );
        return in_dims;
    }
    let mut ideal_w = (in_w * factor).floor() as u32;
    let mut ideal_h = (in_h * factor).floor() as u32;
    if ideal_w % 4 != 0 {
        ideal_w += 4 - (ideal_w % 4);
    }
    if ideal_h % 4 != 0 {
        ideal_h += 4 - (ideal_h % 4);
    }
    (ideal_w, ideal_h)
}

fn find_images(images: &str, im_ext: &str) -> Vec<Vec<SourceImageInfo>> {
    let images = Path::new(&images);
    // unwraping on pattern because a bad pattern is a programming error here
    let mut paths = Vec::new();
    for folder in glob(&images.join("*").to_string_lossy()).unwrap() {
        if let Ok(folder) = folder {
            let folder_spec: FolderSpec =
                std::fs::File::open(folder.join("specs.json"))
                    .map(std::io::BufReader::new)
                    .map(serde_json::from_reader)
                    .unwrap_or(Ok(FolderSpec::empty()))
                    .unwrap_or(FolderSpec::empty());
            let mut images = Vec::new();
            for image in
                glob(&folder.join(format!("*.{}", im_ext)).to_string_lossy())
                    .unwrap()
            {
                if let Ok(image) = image {
                    if image.to_string_lossy().contains(" ") {
                        log::error!(
                            "path should not contain a space: {:?}",
                            image,
                        );
                        std::process::exit(1);
                    }
                    let image_dims = image_dimensions(&image);
                    let orientation = image_exif_orientation(&image);
                    if let Ok(image_dims) = image_dims {
                        log::info!(
                            "Including image {:?} ({}x{})",
                            image,
                            image_dims.0,
                            image_dims.1,
                        );
                        let basename =
                            image.file_name().and_then(std::ffi::OsStr::to_str);
                        let user_req = if basename
                            .map(|name| {
                                folder_spec
                                    .one_portraits()
                                    .iter()
                                    .find(|name2| *name2 == name)
                                    .is_some()
                            })
                            .unwrap_or(false)
                        {
                            LayoutReq::OnePortrait
                        } else {
                            LayoutReq::Nothing
                        };
                        images.push(SourceImageInfo {
                            path: image,
                            dimensions: image_dims,
                            orientation,
                            user_req,
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

fn resize_images(
    im_infos: Vec<Vec<SourceImageInfo>>,
    dpm: f32,
    page_dims: (f32, f32),
    images_path: &Path,
) -> Result<Vec<Vec<ImageInfo>>, Box<dyn Error>> {
    let mut res = Vec::with_capacity(im_infos.len());
    for (ind, im_folder) in im_infos.iter().enumerate() {
        let mut cur_folder = Vec::with_capacity(im_folder.len());
        let folder_path = images_path.join(format!("section_{:02}", ind));
        std::fs::create_dir_all(&folder_path)?;
        for im_info in im_folder {
            let ideal_dims =
                compute_good_dimensions(im_info.dimensions, page_dims, dpm);
            let im_path = &im_info.path;
            let resized_path = folder_path.join(im_path.file_name().unwrap());
            let rotated_dims = match im_info.orientation {
                Orientation::Rotate90 | Orientation::Rotate270 => {
                    (ideal_dims.1, ideal_dims.0)
                }
                _ => ideal_dims,
            };
            cur_folder.push(ImageInfo {
                resize_dims: ideal_dims,
                path: resized_path,
                rotated_dims,
                user_req: im_info.user_req,
            });
        }
        im_folder
            .par_iter()
            .zip(&cur_folder)
            .map(|(source, target)| {
                let im_path = &source.path;
                let resized_path = &target.path;

                // early check if resizing is necessary
                let in_mtime =
                    std::fs::metadata(im_path).and_then(|x| x.modified());
                let out_mtime =
                    std::fs::metadata(resized_path).and_then(|x| x.modified());
                if let (Ok(in_mtime), Ok(out_mtime)) = (in_mtime, out_mtime) {
                    if in_mtime <= out_mtime {
                        log::info!(
                            "no need to resize {:?}, up to date",
                            im_path
                        );
                        return Ok(());
                    }
                }

                let im = image::open(im_path)?;
                log::info!("resizing {:?}", im_path);
                let (w, h) = target.resize_dims;
                let im = im.resize(w, h, image::FilterType::Gaussian);
                let im = match source.orientation {
                    Orientation::Rotate90 => im.rotate90(),
                    Orientation::Rotate180 => im.rotate180(),
                    Orientation::Rotate270 => im.rotate270(),
                    Orientation::Flipped => {
                        log::info!(
                            "Refusing to modify flipped image {:?}",
                            im_path,
                        );
                        im
                    }
                    _ => im,
                };
                // should not have a bad path at this point: SourceImageInfo
                // is trusted
                let mut out_file = std::io::BufWriter::new(
                    std::fs::File::create(&resized_path)?,
                );
                im.write_to(&mut out_file, ImageOutputFormat::JPEG(90))?;
                Ok(())
            })
            .collect::<ImageResult<()>>()?;
        res.push(cur_folder);
    }
    Ok(res)
}

fn replace(
    io_string: &mut String,
    pat: &str,
    replace_with: &str,
) -> Result<(), ()> {
    let pos = io_string.find(pat).ok_or(())?;
    io_string.replace_range(pos..(pos + pat.len()), replace_with);
    Ok(())
}

fn replace_path(
    io_string: &mut String,
    pat: &str,
    im: &ImageInfo,
    page_path: &Path,
) {
    if let Ok(can_path) = im.path.canonicalize() {
        if let Some(im_path) = can_path.to_str() {
            replace(io_string, pat, im_path).unwrap();
        } else {
            log::error!(
                "could not include image path {:?} in {:?}: utf-8 failed",
                im.path,
                page_path,
            );
        }
    } else {
        log::error!(
            "could not include image path {:?} in {:?}: canonicalize failed",
            im.path,
            page_path,
        );
    }
}

struct BookInfo {
    title: String,
}

#[derive(Debug)]
enum PageKind {
    TwoLandscapes,
    OnePortrait,
}

#[derive(Debug)]
struct PageInfo {
    path: PathBuf,
    kind: PageKind,
}

fn write_toplevel(
    out_folder: &Path,
    book_info: &BookInfo,
    page_infos: &[PageInfo],
) -> std::io::Result<()> {
    let mut toplevel_text = include_str!("../data/toplevel.tex").to_string();
    replace(&mut toplevel_text, "PHOTOTEX_TITLE", &book_info.title).unwrap();
    let mut page_includes = String::new();
    for page in page_infos {
        if let Some(path) = page.path.canonicalize()?.to_str() {
            page_includes.push_str(&format!("\\input{{{}}}\n", path));
        } else {
            log::error!("could not include page {:?}", page.path);
        }
    }
    replace(
        &mut toplevel_text,
        "PHOTOTEX_PAGES_INCLUDE_PLACEHOLDER",
        &page_includes,
    )
    .unwrap();
    replace(&mut toplevel_text, "PHOTOTEX_FOURTH_COVER", "").unwrap();

    let top_file_name = "photobook.tex";
    let toplevel_file = out_folder.join(top_file_name);
    let f = std::fs::File::create(&toplevel_file)?;
    let mut writer = std::io::BufWriter::new(f);
    write!(writer, "{}", toplevel_text)?;

    let makefile = out_folder.join("Makefile");
    let mut makefile_text = include_str!("../data/Makefile").to_string();
    replace(
        &mut makefile_text,
        "PHOTOTEX_TOPLEVEL_FILE_NAME",
        top_file_name,
    )
    .unwrap();
    let f = std::fs::File::create(&makefile)?;
    let mut writer = std::io::BufWriter::new(f);
    write!(writer, "{}", makefile_text)?;
    Ok(())
}

fn write_two_landscapes(
    out_folder: &Path,
    page_id: usize,
    im0: &ImageInfo,
    im1: &ImageInfo,
) -> std::io::Result<PageInfo> {
    let page_path = out_folder.join(format!("page{:03}", page_id));
    std::fs::create_dir_all(&page_path)?;
    let page_path = page_path.join("page.tex");
    let f = std::fs::File::create(&page_path)?;
    let mut writer = std::io::BufWriter::new(f);
    let mut page_text =
        include_str!("../data/page_2_landscapes.tex").to_string();
    if let Some(im0_path) = im0.path.canonicalize()?.to_str() {
        replace(&mut page_text, "PHOTOTEX_FIRST_IMAGE_PATH", im0_path).unwrap();
    } else {
        log::error!(
            "could not include image path {:?} in {:?}: utf-8 failed",
            im0.path,
            page_path,
        );
    }
    if let Some(im1_path) = im1.path.canonicalize()?.to_str() {
        replace(&mut page_text, "PHOTOTEX_SECOND_IMAGE_PATH", im1_path)
            .unwrap();
    } else {
        log::error!(
            "could not include image path {:?} in {:?}: utf-8 failed",
            im1.path,
            page_path,
        );
    }
    replace(&mut page_text, "PHOTOTEX_FIRST_LEGEND", "%").unwrap();
    replace(&mut page_text, "PHOTOTEX_SECOND_LEGEND", "%").unwrap();
    write!(writer, "{}", page_text)?;

    Ok(PageInfo {
        path: page_path,
        kind: PageKind::TwoLandscapes,
    })
}

fn write_two_portraits_one_landscape(
    out_folder: &Path,
    page_id: usize,
    im0: &ImageInfo,
    im1: &ImageInfo,
    im2: &ImageInfo,
) -> std::io::Result<PageInfo> {
    let page_path = out_folder.join(format!("page{:03}", page_id));
    std::fs::create_dir_all(&page_path)?;
    let page_path = page_path.join("page.tex");
    let f = std::fs::File::create(&page_path)?;
    let mut writer = std::io::BufWriter::new(f);
    let mut page_text =
        include_str!("../data/page_2_portrait_1_landscape.tex").to_string();
    let (im0_, im1_, im2_);
    if im0.rotated_dims.0 >= im0.rotated_dims.1 {
        im2_ = im0;
        im0_ = im1;
        im1_ = im2;
    } else if im1.rotated_dims.0 >= im1.rotated_dims.1 {
        im2_ = im1;
        im0_ = im0;
        im1_ = im2;
    } else {
        im2_ = im2;
        im0_ = im0;
        im1_ = im1;
    }
    replace_path(&mut page_text, "PHOTOTEX_FIRST_IMAGE_PATH", im0_, &page_path);
    replace_path(&mut page_text, "PHOTOTEX_SECOND_IMAGE_PATH", im1_, &page_path);
    replace_path(&mut page_text, "PHOTOTEX_THIRD_IMAGE_PATH", im2_, &page_path);
    replace(&mut page_text, "PHOTOTEX_FIRST_SECOND_LEGENDS", "%").unwrap();
    replace(&mut page_text, "PHOTOTEX_THIRD_LEGEND", "%").unwrap();
    write!(writer, "{}", page_text)?;

    Ok(PageInfo {
        path: page_path,
        kind: PageKind::TwoLandscapes,
    })
}

fn write_four_portraits(
    out_folder: &Path,
    page_id: usize,
    im0: &ImageInfo,
    im1: &ImageInfo,
    im2: &ImageInfo,
    im3: &ImageInfo,
) -> std::io::Result<PageInfo> {
    let page_path = out_folder.join(format!("page{:03}", page_id));
    std::fs::create_dir_all(&page_path)?;
    let page_path = page_path.join("page.tex");
    let f = std::fs::File::create(&page_path)?;
    let mut writer = std::io::BufWriter::new(f);
    let mut page_text =
        include_str!("../data/page_4_portraits.tex").to_string();
    replace_path(
        &mut page_text, "PHOTOTEX_FIRST_IMAGE_PATH", &im0, &page_path
    );
    replace_path(
        &mut page_text, "PHOTOTEX_SECOND_IMAGE_PATH", &im1, &page_path
    );
    replace_path(
        &mut page_text, "PHOTOTEX_THIRD_IMAGE_PATH", &im2, &page_path
    );
    replace_path(
        &mut page_text, "PHOTOTEX_FOURTH_IMAGE_PATH", &im3, &page_path
    );
    replace(&mut page_text, "PHOTOTEX_FIRST_SECOND_LEGENDS", "%").unwrap();
    replace(&mut page_text, "PHOTOTEX_THIRD_FOURTH_LEGENDS", "%").unwrap();
    write!(writer, "{}", page_text)?;

    Ok(PageInfo {
        path: page_path,
        kind: PageKind::TwoLandscapes,
    })
}

fn write_one_portrait(
    out_folder: &Path,
    page_id: usize,
    im_info: &ImageInfo,
) -> std::io::Result<PageInfo> {
    let page_path = out_folder.join(format!("page{:03}", page_id));
    std::fs::create_dir_all(&page_path)?;
    let page_path = page_path.join("page.tex");
    let f = std::fs::File::create(&page_path)?;
    let mut writer = std::io::BufWriter::new(f);
    let mut page_text = include_str!("../data/page_1_portrait.tex").to_string();
    if let Some(im_path) = im_info.path.canonicalize()?.to_str() {
        replace(&mut page_text, "PHOTOTEX_IMAGE_PATH", im_path).unwrap();
    } else {
        log::error!(
            "could not include image path {:?} in {:?}: utf-8 failed",
            im_info.path,
            page_path,
        );
    }
    replace(&mut page_text, "PHOTOTEX_LEGEND", "%").unwrap();
    write!(writer, "{}", page_text)?;

    Ok(PageInfo {
        path: page_path,
        kind: PageKind::OnePortrait,
    })
}

fn write_pages(
    out_folder: &Path,
    images: &[Vec<ImageInfo>],
) -> std::io::Result<Vec<PageInfo>> {
    let nb_images = images.iter().map(|v| v.len()).sum();
    let mut page_infos = Vec::with_capacity(nb_images);
    let mut page_id = 0;
    for im_group in images {
        let nb_in_group = im_group.len();
        let mut group_infos = Vec::with_capacity(nb_in_group);
        let two_landscapes = im_group
            .iter()
            .enumerate()
            .filter(|(_, im)| im.rotated_dims.0 >= im.rotated_dims.1)
            .tuples();
        let one_portrait = im_group.iter().enumerate().filter(|(_, im)| {
            im.rotated_dims.0 < im.rotated_dims.1
                && im.user_req == LayoutReq::OnePortrait
        });

        let mut processed = Vec::with_capacity(nb_in_group);
        for ((page_order, im0), (im1_id, im1)) in two_landscapes {
            let page_info =
                write_two_landscapes(out_folder, page_id, im0, im1)?;
            group_infos.push((page_order, page_info));
            page_id += 1;
            processed.push(page_order);
            processed.push(im1_id);
        }
        for (page_order, im) in one_portrait {
            let page_info = write_one_portrait(out_folder, page_id, im)?;
            group_infos.push((page_order, page_info));
            page_id += 1;
            processed.push(page_order);
        }
        let processed: std::collections::HashSet<_> = processed.iter().collect();
        let missing: Vec<_> =
            (0..nb_in_group).filter(|i| !processed.contains(i)).collect();
        let mut nb_consec = 0;
        let mut nb_landscape = 0;
        for (missing_id, &page_order) in missing.iter().enumerate() {
            nb_consec += 1;
            let im = &im_group[page_order];
            if im.rotated_dims.0 >= im.rotated_dims.1 {
                nb_landscape += 1;
            }
            let last = missing_id == missing.len() - 1;
            if nb_landscape == 1 && nb_consec == 3 {
                let page_info = write_two_portraits_one_landscape(
                    out_folder, page_id,
                    &im_group[missing[missing_id - 2]],
                    &im_group[missing[missing_id - 1]],
                    &im_group[page_order],
                )?;
                group_infos.push((page_order, page_info));
                page_id += 1;
                nb_consec = 0;
                nb_landscape = 0;
            } else if nb_consec == 4 {
                // there could be one landscape here, but we accept to have
                // it small.
                let page_info = write_four_portraits(
                    out_folder, page_id,
                    &im_group[missing[missing_id - 3]],
                    &im_group[missing[missing_id - 2]],
                    &im_group[missing[missing_id - 1]],
                    &im_group[page_order],
                )?;
                group_infos.push((page_order, page_info));
                page_id += 1;
                nb_consec = 0;
                nb_landscape = 0;
            } else if nb_consec == 1 && last {
                let page_info = write_one_portrait(
                    out_folder, page_id, &im_group[page_order],
                )?;
                group_infos.push((page_order, page_info));
                page_id += 1;
                nb_consec = 0;
                nb_landscape = 0;
            } else if nb_consec == 2 && last {
                let page_info = write_two_landscapes(
                    out_folder, page_id,
                    &im_group[missing[missing_id - 1]],
                    &im_group[page_order],
                )?;
                group_infos.push((page_order, page_info));
                page_id += 1;
                nb_consec = 0;
                nb_landscape = 0;
            } else if nb_consec == 3 && last {
                let page_info = write_two_portraits_one_landscape(
                    out_folder, page_id,
                    &im_group[missing[missing_id - 2]],
                    &im_group[missing[missing_id - 1]],
                    &im_group[page_order],
                )?;
                group_infos.push((page_order, page_info));
                page_id += 1;
                nb_consec = 0;
                nb_landscape = 0;
            } else if last {
                unreachable!()
            }
            // no terminal else as that is the case where we want to loop
        }

        group_infos.sort_by_key(|(id, _)| *id);
        page_infos.extend(group_infos.drain(..).map(|(_, info)| info));

    }
    Ok(page_infos)
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = clap::App::new("phototex")
        .version("0.1")
        .author("Vincent Barrielle <vincent.barrielle@m4x.org>")
        .about("Generates latex files for photo albums")
        .arg(
            clap::Arg::with_name("images")
                .value_name("FOLDER")
                .help("Path to the images selection folders")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("out_folder")
                .short("-o")
                .value_name("OUT_FOLDER")
                .help("Path where the latex should be written. Defaults to .")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("im_ext")
                .long("--image_ext")
                .value_name("IMAGE_EXT")
                .help("Extension of images files. Defaults to 'jpg'")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("dpm")
                .long("--dpm")
                .value_name("DOTS_PER_MM")
                .help("Desired print definition. Defaults to 12dpm (300dpi).")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("page_format")
                .long("--page-format")
                .value_name("PAGE FORMAT")
                .help("Page format. Defaults to A4.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Increase message verbosity"),
        )
        .get_matches();

    let verbosity = matches.occurrences_of("verbosity") as usize;

    stderrlog::new().verbosity(verbosity + 1).init().unwrap();

    let images = matches.value_of("images").unwrap_or_else(|| {
        println!("{}", matches.usage());
        std::process::exit(1);
    });

    let out_folder = Path::new(matches.value_of("out_folder").unwrap_or("."));

    let im_ext = matches.value_of("im_ext").unwrap_or("jpg");

    let dpm = matches.value_of("dpm").unwrap_or("12.").parse()?;

    let page_format = matches.value_of("page_format").unwrap_or("A4");
    let nb_cpus = num_cpus::get_physical();
    log::info!("resizing will be parallelized on {} threads", nb_cpus);
    rayon::ThreadPoolBuilder::new()
        .num_threads(nb_cpus)
        .build_global()
        .unwrap();

    log::info!("Using images path: {}", images);

    let im_infos = find_images(images, im_ext);
    let page_dims = match page_format {
        "A4" => (210., 297.),
        _ => {
            log::error!("unsupported page format {}", page_format);
            std::process::exit(1);
        }
    };
    let images_path = out_folder.join("images");
    std::fs::create_dir_all(&images_path)?;
    let im_infos = resize_images(im_infos, dpm, page_dims, &images_path)?;

    let page_infos = write_pages(out_folder, &im_infos)?;
    let book_info = BookInfo {
        title: "Titre".to_string(),
    };
    write_toplevel(out_folder, &book_info, &page_infos)?;
    Ok(())
}

mod test {
    #[test]
    fn compute_good_dimensions() {
        let (ideal_w, ideal_h) =
            super::compute_good_dimensions((5184, 3456), (210., 297.), 12.);
        assert!(ideal_w % 4 == 0);
        assert!(ideal_h % 4 == 0);
        assert!(ideal_w < 5184);
        assert!(ideal_h < 3456);
    }
}
