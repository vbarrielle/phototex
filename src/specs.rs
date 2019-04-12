use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct FolderSpec {
    title: Option<String>,
    #[serde(default)]
    one_portraits: Vec<String>,
}

impl FolderSpec {
    pub fn empty() -> Self {
        FolderSpec {
            title: None,
            one_portraits: Vec::new(),
        }
    }

    pub fn one_portraits(&self) -> &[String] {
        &self.one_portraits
    }

    pub fn section_title(&self) -> Option<&str> {
        self.title.as_ref().map(|t| t.as_str())
    }

    pub fn load_or_empty(path: &Path) -> Self {
        std::fs::File::open(path)
            .map(std::io::BufReader::new)
            .map(serde_json::from_reader)
            .map(|spec| {
                log::info!("Loaded folder spec: {:?}:\n\t{:?}", path, spec);
                spec
            })
            .unwrap_or(Ok(FolderSpec::empty()))
            .unwrap_or(FolderSpec::empty())
    }
}
