use std::error::Error;
///! Image utility functions
use std::path::Path;

use glob::glob;
use image::{ImageDecoder, ImageOutputFormat, ImageResult};
use rayon::prelude::*;

use crate::specs::FolderSpec;
use crate::{
    FolderInfo, ImageInfo, LayoutReq, Orientation, SourceFolderInfo,
    SourceImageInfo,
};

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

pub fn find_images(images: &str, im_ext: &str) -> Vec<SourceFolderInfo> {
    let images = Path::new(&images);
    // unwraping on pattern because a bad pattern is a programming error here
    let mut folder_infos = Vec::new();
    for folder in glob(&images.join("*").to_string_lossy()).unwrap() {
        if let Ok(folder) = folder {
            let folder_spec =
                FolderSpec::load_or_empty(&folder.join("specs.json"));
            let mut image_infos = Vec::new();
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
                        image_infos.push(SourceImageInfo {
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
            folder_infos.push(SourceFolderInfo {
                image_infos,
                folder_spec,
            });
        } else {
            log::warn!("Ignoring folder {:?}", folder.unwrap_err());
        }
    }
    folder_infos
}

pub fn resize_images(
    folder_infos: Vec<SourceFolderInfo>,
    dpm: f32,
    page_dims: (f32, f32),
    images_path: &Path,
) -> Result<Vec<FolderInfo>, Box<dyn Error>> {
    let mut res = Vec::with_capacity(folder_infos.len());
    for (ind, source_folder) in folder_infos.into_iter().enumerate() {
        let mut image_infos =
            Vec::with_capacity(source_folder.image_infos.len());
        let folder_path = images_path.join(format!("section_{:02}", ind));
        std::fs::create_dir_all(&folder_path)?;
        for im_info in &source_folder.image_infos {
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
            image_infos.push(ImageInfo {
                resize_dims: ideal_dims,
                path: resized_path,
                rotated_dims,
                user_req: im_info.user_req,
            });
        }
        source_folder
            .image_infos
            .par_iter()
            .zip(&image_infos)
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

                let im = image::open(im_path).map_err(|e| {
                    log::error!("error opening image {:?}: {}", im_path, e);
                    e
                })?;
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
        res.push(FolderInfo {
            image_infos,
            folder_spec: source_folder.folder_spec,
        });
    }
    Ok(res)
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
