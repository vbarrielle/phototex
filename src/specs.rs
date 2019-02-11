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
}
