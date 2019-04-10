use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct FolderSpec {
    one_portraits: Vec<String>,
}

impl FolderSpec {
    pub fn empty() -> Self {
        FolderSpec {
            one_portraits: Vec::new(),
        }
    }

    pub fn one_portraits(&self) -> &[String] {
        &self.one_portraits
    }

    pub fn load_or_empty(path: &Path) -> Self {
        std::fs::File::open(path)
            .map(std::io::BufReader::new)
            .map(serde_json::from_reader)
            .unwrap_or(Ok(FolderSpec::empty()))
            .unwrap_or(FolderSpec::empty())
    }
}
