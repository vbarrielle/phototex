use std::path::{Path, PathBuf};

pub mod book_structure;
pub mod im_handling;
mod pages;
pub mod pdf_handling;
pub mod specs;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum LayoutReq {
    OnePortrait,
    Nothing,
}

#[derive(Debug)]
pub struct SourceFolderInfo {
    folder_spec: specs::FolderSpec,
    image_infos: Vec<SourceImageInfo>,
}

#[derive(Debug)]
pub struct SourceImageInfo {
    path: PathBuf,
    dimensions: (u32, u32),
    orientation: Orientation,
    user_req: LayoutReq,
}

#[derive(Debug)]
pub struct FolderInfo {
    pub folder_spec: specs::FolderSpec,
    pub image_infos: Vec<ImageInfo>,
}

#[derive(Debug)]
pub struct ImageInfo {
    pub path: PathBuf,
    resize_dims: (u32, u32),
    rotated_dims: (u32, u32),
    user_req: LayoutReq,
}

#[derive(Copy, Clone, Debug)]
pub enum PageOrientation {
    Portrait,
    Landscape,
    Square,
}

impl std::fmt::Display for PageOrientation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PageOrientation::Portrait => "portrait",
            PageOrientation::Landscape => "landscape",
            PageOrientation::Square => "square",
        }
        .fmt(f)
    }
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

#[derive(Copy, Clone)]
pub struct BookInfo<'a> {
    pub title: &'a str,
    pub title_font_size: &'a str,
    pub title_leading_size: &'a str,
    pub title_im_path: Option<&'a Path>,
}

#[derive(Debug)]
enum PageKind {
    TwoLandscapes,
    OnePortrait,
}

#[derive(Debug)]
pub struct PageInfo {
    path: PathBuf,
    kind: PageKind,
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
