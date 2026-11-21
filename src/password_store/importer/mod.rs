pub mod bitwarden;

use super::EntryField;

#[derive(Debug, Clone)]
pub struct ImportEntry {
    pub name: String,
    pub password: Option<String>,
    pub fields: Vec<EntryField>,
    pub otp_uri: Option<String>,
    pub notes: Option<String>,
    pub folder: Option<String>,
}

pub trait Importer: std::fmt::Debug {
    fn parse(&self, data: &str) -> Result<Vec<ImportEntry>, ImportError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("the import file is encrypted; decrypt it first")]
    EncryptedFile,

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("import file contains no recognizable entries")]
    NoEntries,

    #[error("import item {0} has no valid name")]
    InvalidItemName(usize),

    #[error("import item {0} has a multiline password that this store format cannot represent")]
    MultilinePassword(usize),

    #[error("import item {0} has a field that cannot be stored unchanged")]
    InvalidField(usize),
}
