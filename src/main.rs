use std::error::Error;
use std::path::Path;

use phototex::book_structure;
use phototex::im_handling;
use phototex::pdf_handling;
use phototex::BookInfo;

fn main() -> Result<(), Box<dyn Error>> {
    let matches = clap::App::new("phototex")
        .version("0.1")
        .author("Vincent Barrielle <vincent.barrielle@m4x.org>")
        .about("Generates latex files for photo albums.")
        .arg(
            clap::Arg::with_name("images")
                .value_name("FOLDER")
                .help("Path to the images selection folders.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("out_folder")
                .short("-o")
                .long("--output_folder")
                .value_name("OUT_FOLDER")
                .help(
                    "Path where the latex should be written. Defaults to '.'.",
                )
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
            clap::Arg::with_name("title")
                .long("--title")
                .value_name("TITLE")
                .help("Title of the album. Defaults to \"\".")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("title_font_size")
                .long("--title-font-size")
                .value_name("TITLE_FONT_SIZE")
                .help("Font size for the title. Defaults to 42pt.")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("title_im_name")
                .long("--title-image-name")
                .value_name("TITLE_IMAGE_NAME")
                .help(
                    "Name of the optional image for the title page (with ext).",
                )
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("page_format")
                .long("--page-format")
                .value_name("PAGE_FORMAT")
                .help(
                    "Page format. Defaults to A4 portrait. No support for \
                     other formats presently.",
                )
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("strip_inner_covers")
                .long("--strip-inner-covers")
                .help(
                    "With this flag, a version without inner covers will also \
                     be generated. This can be the required format for  some \
                     print shops.",
                )
                .takes_value(false),
        )
        .arg(
            clap::Arg::with_name("verbosity")
                .short("v")
                .multiple(true)
                .help("Increase message verbosity."),
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

    let strip_inner_covers = matches.is_present("strip_inner_covers");

    let title = matches.value_of("title").unwrap_or("");

    let title_font_size: f32 = matches
        .value_of("title_font_size")
        .unwrap_or("42")
        .parse()?;
    let title_leading_size = title_font_size * 1.10f32;
    let title_font_size = format!("{}pt", title_font_size);
    let title_leading_size = format!("{}pt", title_leading_size);

    let title_im_name = matches.value_of("title_im_name");

    let nb_cpus = num_cpus::get_physical();
    log::info!("resizing will be parallelized on {} threads", nb_cpus);
    rayon::ThreadPoolBuilder::new()
        .num_threads(nb_cpus)
        .build_global()
        .unwrap();

    log::info!("Using images path: {}", images);

    let folder_infos = im_handling::find_images(images, im_ext);
    let page_dims = match page_format {
        "A4" => (210., 297.),
        _ => {
            log::error!("unsupported page format {}", page_format);
            std::process::exit(1);
        }
    };
    let images_path = out_folder.join("images");
    std::fs::create_dir_all(&images_path)?;
    let folder_infos =
        im_handling::resize_images(folder_infos, dpm, page_dims, &images_path)?;
    let title_im_path = title_im_name.and_then(|name| {
        for im_info_folder in &folder_infos {
            for im_info in &im_info_folder.image_infos {
                if im_info.path.ends_with(name) {
                    return Some(im_info.path.as_path());
                }
            }
        }
        None
    });

    let page_infos = book_structure::write_pages(out_folder, &folder_infos)?;
    let book_info = BookInfo {
        title,
        title_font_size: &title_font_size,
        title_leading_size: &title_leading_size,
        title_im_path,
    };
    let top_file_name =
        book_structure::write_toplevel(out_folder, book_info, &page_infos)?;

    let pdf_file_name = pdf_handling::generate_pdf(out_folder, &top_file_name)?;
    if strip_inner_covers {
        log::info!("Stripping inner covers...");
        let trimmed_pdf_file_name = pdf_handling::remove_second_third_covers(
            out_folder,
            &pdf_file_name,
        )?;
        log::info!("Stripping done, in {}", trimmed_pdf_file_name);
    }
    Ok(())
}
