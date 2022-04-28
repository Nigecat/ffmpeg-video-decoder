/// An error from the decoder
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum DecodeError {
    #[error("unable to open input data")]
    UnableToOpenInput,
    #[error("unable to read stream information")]
    UnableToReadStreamInfo,
    /// This may mean that the source data does not have an active video stream
    #[error("unable to find video stream")]
    UnableToFindVideoStream,
    /// The target codec is not supported by ffmpeg
    #[error("unsupported codec (by ffmpeg)")]
    UnsupportedCodec,
    #[error("could not read frame buffer")]
    UnableToReadFrameBuffer,
}
