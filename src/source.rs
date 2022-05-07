use std::path::PathBuf;

/// The input data for the decoder
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum VideoSource {
    /// Raw binary data
    Raw(Vec<u8>),
    /// A path to a file
    Filesystem(PathBuf),
}

impl From<PathBuf> for VideoSource {
    fn from(path: PathBuf) -> Self {
        Self::Filesystem(path)
    }
}

impl From<Vec<u8>> for VideoSource {
    fn from(data: Vec<u8>) -> Self {
        Self::Raw(data)
    }
}

impl From<String> for VideoSource {
    fn from(s: String) -> Self {
        PathBuf::from(s).into()
    }
}

impl From<&str> for VideoSource {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}
