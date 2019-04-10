use std::io::Write;
///! This module contains functions to write pages with various layouts
use std::path::Path;

use crate::{replace, replace_path, ImageInfo, PageInfo, PageKind};

pub(crate) fn write_two_landscapes(
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

pub(crate) fn write_two_portraits_one_landscape(
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
