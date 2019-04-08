use std::path::Path;
use std::process::Command;

pub fn generate_pdf(
    output_folder: &Path, tex_file_name: &str
) -> std::io::Result<String>
{
    let mut pdflatex = Command::new("pdflatex");
    pdflatex.arg(tex_file_name).current_dir(output_folder);
    log::info!("first call to pdflatex");
    let output = pdflatex.output();
    if output.is_err() {
        log::error!("Could not launch pdflatex, is it correctly installed?");
        return output.map(|_| String::new());
    }
    if !output.unwrap().status.success() {
        log::error!("Latex compilation error");
        // TODO write log to file
    }
    log::info!("second call to pdflatex");
    let output = pdflatex.output().expect("pdflatex failed to execute twice");
    if !output.status.success() {
        log::error!("Latex compilation error");
    }
    let pdf_file_name = if tex_file_name.ends_with(".tex") {
        tex_file_name.replace(".tex", ".pdf")
    } else {
        format!("{}.pdf", tex_file_name)
    };
    Ok(pdf_file_name)
}

pub fn remove_second_third_covers(
    output_folder: &Path,
    full_pdf_file_name: &str,
) -> std::io::Result<String>
{
    let pdf_path = output_folder.join(full_pdf_file_name);
    let mut pdf = lopdf::Document::load(pdf_path)?;
    let page_numbers: Vec<_> = pdf.get_pages().keys().cloned().collect();
    if page_numbers.len() <= 6 {
        log::error!("incorrect number of pages for a book");
        return Err(
            std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof, "pdf is too short",
            )
        );
    }
    let to_delete = [page_numbers[1], page_numbers[page_numbers.len() - 2]];
    pdf.delete_pages(&to_delete);
    let trimmed_pdf_file_name = full_pdf_file_name
        .replace(".pdf", "_trimmed.pdf");
    pdf.save(output_folder.join(&trimmed_pdf_file_name))?;
    Ok(trimmed_pdf_file_name)
}
