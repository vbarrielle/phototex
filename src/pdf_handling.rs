use std::path::Path;
use std::process::Command;

pub fn generate_pdf(output_folder: &Path, tex_file_name: &str) -> std::io::Result<()>
{
    let mut pdflatex = Command::new("pdflatex");
    pdflatex.arg(tex_file_name).current_dir(output_folder);
    log::info!("first call to pdflatex");
    let output = pdflatex.output();
    if output.is_err() {
        log::error!("Could not launch pdflatex, is it correctly installed?");
        return output.map(|_| ());
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
    Ok(())
}
