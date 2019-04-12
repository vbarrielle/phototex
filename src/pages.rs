use std::io::{Read, Write, BufReader, BufWriter};
use std::fs::File;
///! This module contains functions to write pages with various layouts
use std::path::{Path, PathBuf};

use crate::{replace, replace_path, ImageInfo, PageInfo, PageKind};

#[derive(Debug)]
pub struct Page {
    path: PathBuf,
}

pub(crate) fn set_section_title(
    page_info: &PageInfo, title: Option<&str>
) -> std::io::Result<()> {
    let file = File::open(&page_info.path)?;
    let mut buf_reader = BufReader::new(file);
    let mut page_text = String::new();
    buf_reader.read_to_string(&mut page_text)?;
    let comment = "% page title";
    replace(
        &mut page_text,
        "PHOTOTEX_PAGE_TITLE",
        title.unwrap_or(comment),
    )
    .unwrap();

    let f = File::create(&page_info.path)?;
    let mut writer = BufWriter::new(f);
    write!(writer, "{}", page_text)?;
    Ok(())
}

impl Page {
    pub fn new(
        page_id: &mut usize,
        out_folder: &Path,
    ) -> Page {
        let path = out_folder.join(format!("page{:03}", *page_id));
        *page_id += 1;
        Page {
            path,
        }
    }

    pub(crate) fn write_two_landscapes(
        self,
        im0: &ImageInfo,
        im1: &ImageInfo,
    ) -> std::io::Result<PageInfo> {
        let page_path = &self.path;
        std::fs::create_dir_all(page_path)?;
        let page_path = page_path.join("page.tex");
        let f = File::create(&page_path)?;
        let mut writer = BufWriter::new(f);
        let mut page_text =
            include_str!("../data/page_2_landscapes.tex").to_string();
        if let Some(im0_path) = im0.path.canonicalize()?.to_str() {
            replace(&mut page_text, "PHOTOTEX_FIRST_IMAGE_PATH", im0_path)
                .unwrap();
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

    pub(crate) fn write_two_portraits_one_landscape(
        self,
        im0: &ImageInfo,
        im1: &ImageInfo,
        im2: &ImageInfo,
    ) -> std::io::Result<PageInfo> {
        let page_path = &self.path;
        std::fs::create_dir_all(page_path)?;
        let page_path = page_path.join("page.tex");
        let f = File::create(&page_path)?;
        let mut writer = BufWriter::new(f);
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
        replace_path(
            &mut page_text,
            "PHOTOTEX_FIRST_IMAGE_PATH",
            im0_,
            &page_path,
        );
        replace_path(
            &mut page_text,
            "PHOTOTEX_SECOND_IMAGE_PATH",
            im1_,
            &page_path,
        );
        replace_path(
            &mut page_text,
            "PHOTOTEX_THIRD_IMAGE_PATH",
            im2_,
            &page_path,
        );
        replace(&mut page_text, "PHOTOTEX_FIRST_SECOND_LEGENDS", "%").unwrap();
        replace(&mut page_text, "PHOTOTEX_THIRD_LEGEND", "%").unwrap();
        write!(writer, "{}", page_text)?;

        Ok(PageInfo {
            path: page_path,
            kind: PageKind::TwoLandscapes,
        })
    }

    pub(crate) fn write_four_portraits(
        self,
        im0: &ImageInfo,
        im1: &ImageInfo,
        im2: &ImageInfo,
        im3: &ImageInfo,
    ) -> std::io::Result<PageInfo> {
        let page_path = &self.path;
        std::fs::create_dir_all(page_path)?;
        let page_path = page_path.join("page.tex");
        let f = File::create(&page_path)?;
        let mut writer = BufWriter::new(f);
        let mut page_text =
            include_str!("../data/page_4_portraits.tex").to_string();
        replace_path(
            &mut page_text,
            "PHOTOTEX_FIRST_IMAGE_PATH",
            &im0,
            &page_path,
        );
        replace_path(
            &mut page_text,
            "PHOTOTEX_SECOND_IMAGE_PATH",
            &im1,
            &page_path,
        );
        replace_path(
            &mut page_text,
            "PHOTOTEX_THIRD_IMAGE_PATH",
            &im2,
            &page_path,
        );
        replace_path(
            &mut page_text,
            "PHOTOTEX_FOURTH_IMAGE_PATH",
            &im3,
            &page_path,
        );
        replace(&mut page_text, "PHOTOTEX_FIRST_SECOND_LEGENDS", "%").unwrap();
        replace(&mut page_text, "PHOTOTEX_THIRD_FOURTH_LEGENDS", "%").unwrap();
        write!(writer, "{}", page_text)?;

        Ok(PageInfo {
            path: page_path,
            kind: PageKind::TwoLandscapes,
        })
    }

    pub(crate) fn write_one_portrait(
        self,
        im_info: &ImageInfo,
    ) -> std::io::Result<PageInfo> {
        let page_path = &self.path;
        std::fs::create_dir_all(page_path)?;
        let page_path = page_path.join("page.tex");
        let f = File::create(&page_path)?;
        let mut writer = BufWriter::new(f);
        let mut page_text =
            include_str!("../data/page_1_portrait.tex").to_string();
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
}
