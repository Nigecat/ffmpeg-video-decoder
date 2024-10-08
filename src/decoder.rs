use super::{DecodeError, Dimensions, VideoSource};
use crate::c::{path_to_raw, read_stream, Stream};
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::{ffi, mem, ptr};

/// ffmpeg buffer size
const BUFFER_SIZE: usize = 8192;

// ffmpeg buffer alignment (this is unrelated to the previous constant)
const BUFFER_ALIGNMENT: std::ffi::c_int = 32; // 256 bits

/// A single frame from a decoded video
pub struct Frame {
    index: usize,
    data: Vec<u8>,
    dimensions: Dimensions,
}

impl Frame {
    /// The frame number in the source video (starts at 1)
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get the dimensions of the frame data
    #[inline]
    pub fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    /// Get a reference to the raw frame data
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Convert this frame into its raw data
    #[inline]
    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    /// The raw data along with its dimensions
    #[inline]
    pub fn into_raw(self) -> (Vec<u8>, Dimensions) {
        (self.data, self.dimensions)
    }

    /// Convert this frame into a [image::DynamicImage](https://docs.rs/image/latest/image/enum.DynamicImage.html)
    #[cfg(feature = "image")]
    pub fn into_image(self) -> image::DynamicImage {
        image::DynamicImage::ImageRgb8(
            image::ImageBuffer::from_raw(self.dimensions.width, self.dimensions.height, self.data)
                .unwrap(), // unwrap is safe as both data and dimensions are readonly to the caller
        )
    }
}

/// A video decoder
///
/// ## Usage
///
/// Simply create a [`VideoDecoder`] then call [`VideoDecoder::next_frame`]:
///
/// ```rust
/// # fn main() {
/// use ffmpeg_video_decoder::VideoDecoder;
///
/// let file = "video.mp4";
/// # let file = "test.mp4";
/// let mut decoder = VideoDecoder::new(file, false).unwrap();
/// let first_frame = decoder.next_frame().unwrap();
/// let second_frame = decoder.next_frame().unwrap();
/// // etc...
/// # }
/// ```
///
/// <br>
///
/// Alternatively, a `while let` loop can be used to iterate over frames:
///
/// ```rust
/// # fn main() {
/// use ffmpeg_video_decoder::VideoDecoder;
///
/// let file = "video.mp4";
/// # let file = "test.mp4";
/// let mut decoder = VideoDecoder::new(file, false).unwrap();
/// while let Some(next_frame) = decoder.next_frame().unwrap() {
///     // do something with the frame
/// }
/// # }
/// ```
pub struct VideoDecoder {
    /// The framerate of the decoded video
    framerate: f32,
    /// The dimensions of the the decoded video
    dimensions: Dimensions,
    /// Internal frame buffer, as ffmpeg returns frames in chunks
    buffer: VecDeque<Frame>,
    /// Whether we should loop the frames when we reach the end of the input data
    should_loop: bool,
    /// The next frame index
    index: usize,

    /// The source data, we must store it so the pointer passed to ffmpeg is not dropped
    _source: VideoSource,

    // -------------- ffmpeg data --------------
    texture_data: Vec<u8>,
    sws_context: *mut ffmpeg::SwsContext,
    rgb_frame: *mut ffmpeg::AVFrame,
    raw_frame: *mut ffmpeg::AVFrame,
    /// Only used if we got a [`VideoSource::Raw(_)`]
    avio: Option<*mut ffmpeg::AVIOContext>,
    codec_ctx: *mut ffmpeg::AVCodecContext,
    input_ctx: *mut ffmpeg::AVFormatContext,
    packet: ffmpeg::AVPacket,
    stream_id: i32,
}

impl VideoDecoder {
    /// Create a new video decoder.
    ///
    /// # Arguments
    ///
    /// * `source` - The input video data
    /// * `should_loop` - Whether the decoder should loop back to the start once reaching the end of the source data
    pub fn new<S>(source: S, should_loop: bool) -> Result<Self, DecodeError>
    where
        S: Into<VideoSource>,
    {
        let source: VideoSource = source.into();

        unsafe {
            let buffer = ffmpeg::av_malloc(BUFFER_SIZE);

            let mut avio: Option<*mut ffmpeg::AVIOContext> = None;
            let mut input_ctx: *mut ffmpeg::AVFormatContext = ffmpeg::avformat_alloc_context();

            if let VideoSource::Raw(ref data) = source {
                let mut stream = Stream {
                    length: data.len(),
                    offset: 0,
                    data: data.as_ptr(),
                };

                avio = Some(ffmpeg::avio_alloc_context(
                    buffer as *mut u8,
                    BUFFER_SIZE as i32,
                    0,
                    &mut stream as *mut Stream as *mut ffi::c_void,
                    Some(read_stream),
                    None,
                    None,
                ));

                (*input_ctx).pb = avio.unwrap();
                (*input_ctx).flags |= ffmpeg::AVFMT_FLAG_CUSTOM_IO;
            }

            let mut _source_path_raw = Vec::new();
            let path = match source {
                VideoSource::Raw(_) => ptr::null(),
                VideoSource::Filesystem(ref path) => {
                    _source_path_raw = path_to_raw(path).ok_or(DecodeError::InvalidSource)?;
                    _source_path_raw.as_ptr()
                }
            };

            // Open video
            if ffmpeg::avformat_open_input(
                &mut input_ctx,
                path as *const i8,
                ptr::null_mut(),
                ptr::null_mut(),
            ) != 0
            {
                return Err(DecodeError::UnableToOpenInput);
            }

            // Get stream information
            if ffmpeg::avformat_find_stream_info(input_ctx, ptr::null_mut()) < 0 {
                return Err(DecodeError::UnableToReadStreamInfo);
            }

            // Find video stream
            let (codecpar, stream_id) = {
                let mut stream_id = None;

                for i in 0..(*input_ctx).nb_streams as isize {
                    if (*(*(*(*input_ctx).streams.offset(i))).codecpar).codec_type
                        == ffmpeg::AVMediaType::AVMEDIA_TYPE_VIDEO
                    {
                        stream_id = Some(i);
                        break;
                    }
                }

                let stream_id = stream_id.ok_or(DecodeError::UnableToFindVideoStream)?;
                (
                    (*(*(*input_ctx).streams.offset(stream_id))).codecpar,
                    stream_id,
                )
            };

            let codec = ffmpeg::avcodec_find_decoder((*codecpar).codec_id);
            if codec.is_null() {
                return Err(DecodeError::UnsupportedCodec);
            }

            // Duplicate codec so we can reuse the input context
            let codec_ctx = {
                let codec_ctx = ffmpeg::avcodec_alloc_context3(codec);
                ffmpeg::avcodec_parameters_to_context(codec_ctx, codecpar);
                codec_ctx
            };

            // Open decoder context
            if ffmpeg::avcodec_open2(codec_ctx, codec, ptr::null_mut()) < 0 {
                return Err(DecodeError::UnsupportedCodec);
            }

            // Allocate frame buffers
            let raw_frame = ffmpeg::av_frame_alloc();
            let rgb_frame = ffmpeg::av_frame_alloc();

            let buffer_size = ffmpeg::av_image_get_buffer_size(
                ffmpeg::AVPixelFormat::AV_PIX_FMT_RGB24,
                (*codec_ctx).width,
                (*codec_ctx).height,
                BUFFER_ALIGNMENT,
            ) as usize;

            let mut texture_data: Vec<u8> = vec![0; buffer_size];

            if ffmpeg::av_image_fill_arrays(
                (*rgb_frame).data.as_mut_ptr(),
                (*rgb_frame).linesize.as_mut_ptr(),
                texture_data.as_mut_ptr(),
                ffmpeg::AVPixelFormat::AV_PIX_FMT_RGB24,
                (*codec_ctx).width,
                (*codec_ctx).height,
                BUFFER_ALIGNMENT,
            ) <= 0
            {
                return Err(DecodeError::UnableToReadFrameBuffer);
            }

            // Creater converter context
            let sws_context = ffmpeg::sws_getContext(
                (*codec_ctx).width,                      // Source
                (*codec_ctx).height,                     // Source
                (*codec_ctx).pix_fmt,                    // Source
                (*codec_ctx).width,                      // Destination
                (*codec_ctx).height,                     // Destination
                ffmpeg::AVPixelFormat::AV_PIX_FMT_RGB24, // Destination
                ffmpeg::SWS_BILINEAR,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );

            let packet: ffmpeg::AVPacket = mem::zeroed();
            let dimensions = ((*codec_ctx).width as u32, (*codec_ctx).height as u32);
            let framerate = (**(*input_ctx).streams).r_frame_rate;
            let framerate = framerate.num as f32 / framerate.den as f32;

            Ok(VideoDecoder {
                dimensions: Dimensions {
                    width: dimensions.0,
                    height: dimensions.1,
                },
                framerate,
                codec_ctx,
                input_ctx,
                texture_data,
                index: 1, // first frame is frame 1
                sws_context,
                rgb_frame,
                raw_frame,
                _source: source,
                avio,
                packet,
                buffer: VecDeque::new(),
                should_loop,
                stream_id: stream_id as i32,
            })
        }
    }

    /// Get the next frame from the input, if [`VideoDecoder::will_loop`] is `true` then this is guaranteed to never return `Ok(None)`.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, DecodeError> {
        if let Some(next) = self.buffer.pop_front() {
            return Ok(Some(next));
        }

        unsafe {
            let next_frame = ffmpeg::av_read_frame(self.input_ctx, &mut self.packet);
            if next_frame < 0 {
                // out of frames
                if self.should_loop {
                    self.loop_ctx();
                    return self.next_frame();
                } else {
                    return Ok(None);
                }
            }

            // Check that this packet is in the right stream
            if self.packet.stream_index == self.stream_id {
                if ffmpeg::avcodec_send_packet(self.codec_ctx, &self.packet) < 0 {
                    return Err(DecodeError::UnableToSendPacketToDecoder);
                }

                // Decode packet frames
                while ffmpeg::avcodec_receive_frame(self.codec_ctx, self.raw_frame) >= 0 {
                    // Convert frame to RGB24
                    ffmpeg::sws_scale(
                        self.sws_context,
                        (*self.raw_frame).data.as_ptr() as *const *const _,
                        (*self.raw_frame).linesize.as_ptr() as *mut _,
                        0,
                        (*self.codec_ctx).height as std::os::raw::c_int,
                        (*self.rgb_frame).data.as_ptr(),
                        (*self.rgb_frame).linesize.as_ptr() as *mut _,
                    );

                    // Add to frame buffer
                    self.buffer.push_back(Frame {
                        index: self.index,
                        data: self.texture_data.clone(),
                        dimensions: self.dimensions,
                    });
                    self.index += 1;
                }
            }

            ffmpeg::av_packet_unref(&mut self.packet);
        }

        self.next_frame()
    }

    /// Skip the next `n` frames.
    ///
    /// Note that this function will never loop (even if [`VideoDecoder::will_loop`] is `true`).
    pub fn skip(&mut self, n: isize) {
        let frames = n;

        match frames.cmp(&0) {
            Ordering::Greater => {
                let mut frames = frames as usize;

                // Clear frame buffer
                if !self.buffer.is_empty() && self.buffer.len() <= frames {
                    let limit = match self.buffer.len() <= frames {
                        true => self.buffer.len(),
                        false => frames,
                    };

                    self.buffer.drain(..=limit);
                    frames -= limit;
                }

                while frames > 0 {
                    unsafe {
                        let next_frame = ffmpeg::av_read_frame(self.input_ctx, &mut self.packet);
                        if next_frame < 0 {
                            // out of frames
                            return;
                        }

                        // Check that this packet is in the right stream
                        if self.packet.stream_index == self.stream_id {
                            if ffmpeg::avcodec_send_packet(self.codec_ctx, &self.packet) < 0 {
                                // If we can't decode the packet, ignore it
                                // (this does not count toward the skipped frames, but this may change in the future)
                                continue;
                            }

                            // Read packet frames
                            while ffmpeg::avcodec_receive_frame(self.codec_ctx, self.raw_frame) >= 0
                            {
                                frames -= 1;

                                // Update frame index
                                self.index += 1;

                                // Packet may contain multiple frames,
                                //      so we need to check every frame to prevent this from underflowing
                                if frames == 0 {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Ordering::Less => {
                // optimize there is probably a better way to do this

                // Determine how many frames we need go to from the start
                let offset = self
                    .index
                    // This is needed to account for the fact that the index is the index for the next frame
                    .saturating_sub(2)
                    .saturating_sub(frames.unsigned_abs());

                // Reset to start
                self.loop_ctx();

                // Skip offset
                if offset > isize::MAX as usize {
                    self.skip(isize::MAX);
                    self.skip((offset - isize::MAX as usize) as isize);
                } else {
                    self.skip(offset as isize);
                }
            }
            Ordering::Equal => (),
        }
    }

    /// Get the dimensions of the video
    #[inline]
    pub fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    /// Get the framerate of the video (in frames-per-second)
    #[inline]
    pub fn framerate(&self) -> f32 {
        self.framerate
    }

    /// Check whether the decoder will loop once reaching the end of the source data
    ///
    /// This will be whatever value was passed to [`VideoDecoder::new`].
    /// ```rust
    /// # fn main() {
    /// # use ffmpeg_video_decoder::VideoDecoder;
    ///  let decoder = VideoDecoder::new("test.mp4", true).unwrap();
    ///  assert_eq!(decoder.will_loop(), true);
    /// # }
    /// ```
    /// ```rust
    /// # fn main() {
    /// # use ffmpeg_video_decoder::VideoDecoder;
    ///  let decoder = VideoDecoder::new("test.mp4", false).unwrap();
    ///  assert_eq!(decoder.will_loop(), false);
    /// # }
    /// ```
    #[inline]
    pub fn will_loop(&self) -> bool {
        self.should_loop
    }

    /// Loop the internal decoder context, this will reset the video to the first frame.
    fn loop_ctx(&mut self) {
        unsafe {
            // Seek stream to start
            let stream = (*self.input_ctx).streams.offset(self.stream_id as isize);
            ffmpeg::avio_seek((*self.input_ctx).pb, 0, 0);
            ffmpeg::avformat_seek_file(
                self.input_ctx,
                self.stream_id,
                0,
                0,
                (*(*stream)).duration,
                0,
            );
        }

        // Reset index
        self.index = 1;
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        unsafe {
            ffmpeg::sws_freeContext(self.sws_context);
            ffmpeg::av_free(self.rgb_frame as *mut ffi::c_void);
            ffmpeg::av_free(self.raw_frame as *mut ffi::c_void);
            if let Some(mut avio) = self.avio {
                ffmpeg::avio_context_free(&mut avio);
            }
            ffmpeg::avcodec_close(self.codec_ctx);
            ffmpeg::avcodec_free_context(&mut self.codec_ctx);
            ffmpeg::avformat_close_input(&mut self.input_ctx);
        }
    }
}
