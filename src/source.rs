use std::borrow::Cow;
use std::path::{Path, PathBuf};

/// The input data for the decoder
#[non_exhaustive]
pub enum VideoSource<'source> {
    /// Raw binary data
    Raw(Cow<'source, [u8]>),
    /// A path to a file
    Filesystem(Cow<'source, Path>),
}

impl<'source> From<&'source Path> for VideoSource<'source> {
    fn from(path: &'source Path) -> Self {
        Self::Filesystem(Cow::Borrowed(path))
    }
}

impl From<PathBuf> for VideoSource<'_> {
    fn from(path: PathBuf) -> Self {
        Self::Filesystem(Cow::Owned(path))
    }
}

impl<'source> From<&'source [u8]> for VideoSource<'source> {
    fn from(data: &'source [u8]) -> Self {
        Self::Raw(Cow::Borrowed(data))
    }
}

impl From<Vec<u8>> for VideoSource<'_> {
    fn from(data: Vec<u8>) -> Self {
        Self::Raw(Cow::Owned(data))
    }
}
