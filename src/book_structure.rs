use std::io::Write;
///! Main writing functions for the book
use std::path::Path;

use itertools::Itertools;

use crate::pages;
use crate::replace;
use crate::BookInfo;
use crate::ImageInfo;
use crate::LayoutReq;
use crate::PageInfo;

fn handle_title_image(
    toplevel_text: &mut String,
    im_path: Option<&Path>,
) -> std::io::Result<()> {
    if let Some(im_path) = im_path {
        if let Some(im_path) = im_path.canonicalize()?.to_str() {
            replace(
                toplevel_text,
                "PHOTOTEX_TITLE_IMAGE_COMMAND",
                &format!(
                    "\\includegraphics[width=0.90\\textwidth,\
                     height=0.70\\textheight,\
                     keepaspectratio]{{{}}}",
                    im_path,
                ),
            )
            .unwrap();
            Ok(())
        } else {
            log::error!(
                "could not include image path {:?} in title page: utf-8 failed",
                im_path,
            );
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "utf-8 error",
            ))
        }
    } else {
        replace(toplevel_text, "PHOTOTEX_TITLE_IMAGE_COMMAND", "").unwrap();
        Ok(())
    }
}

pub fn write_toplevel(
    out_folder: &Path,
    book_info: BookInfo,
    page_infos: &[PageInfo],
) -> std::io::Result<String> {
    let mut toplevel_text = include_str!("../data/toplevel.tex").to_string();
    handle_title_image(&mut toplevel_text, book_info.title_im_path)?;
    replace(&mut toplevel_text, "PHOTOTEX_TITLE_STRING", book_info.title)
        .unwrap();
    replace(
        &mut toplevel_text,
        "PHOTOTEX_TITLE_FONT_SIZE",
        book_info.title_font_size,
    )
    .unwrap();
    replace(
        &mut toplevel_text,
        "PHOTOTEX_TITLE_LEADING_SIZE",
        book_info.title_leading_size,
    )
    .unwrap();
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
    Ok(top_file_name.into())
}

pub fn write_pages(
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
                pages::write_two_landscapes(out_folder, page_id, im0, im1)?;
            group_infos.push((page_order, page_info));
            page_id += 1;
            processed.push(page_order);
            processed.push(im1_id);
        }
        for (page_order, im) in one_portrait {
            let page_info = pages::write_one_portrait(out_folder, page_id, im)?;
            group_infos.push((page_order, page_info));
            page_id += 1;
            processed.push(page_order);
        }
        let processed: std::collections::HashSet<_> =
            processed.iter().collect();
        let missing: Vec<_> = (0..nb_in_group)
            .filter(|i| !processed.contains(i))
            .collect();
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
                let page_info = pages::write_two_portraits_one_landscape(
                    out_folder,
                    page_id,
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
                let page_info = pages::write_four_portraits(
                    out_folder,
                    page_id,
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
                let page_info = pages::write_one_portrait(
                    out_folder,
                    page_id,
                    &im_group[page_order],
                )?;
                group_infos.push((page_order, page_info));
                page_id += 1;
                nb_consec = 0;
                nb_landscape = 0;
            } else if nb_consec == 2 && last {
                let page_info = pages::write_two_landscapes(
                    out_folder,
                    page_id,
                    &im_group[missing[missing_id - 1]],
                    &im_group[page_order],
                )?;
                group_infos.push((page_order, page_info));
                page_id += 1;
                nb_consec = 0;
                nb_landscape = 0;
            } else if nb_consec == 3 && last {
                let page_info = pages::write_two_portraits_one_landscape(
                    out_folder,
                    page_id,
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
