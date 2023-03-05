mod c;
mod decoder;
mod error;
mod source;

#[cfg(feature = "image")]
pub use image;

pub use decoder::{Frame, VideoDecoder};
pub use error::DecodeError;
pub use source::VideoSource;

/// The height and width of something
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    width: u32,
    height: u32,
}

impl Dimensions {
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }
}
