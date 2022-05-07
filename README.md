# ffmpeg-video-decoder
A simple (safe) wrapper around ffmpeg-sys to provide a basic video decoder.

## Installation

Since this crate is not in the repository index, the dependency must be added as a git dependency as follows:

```toml
[dependencies]
ffmpeg-video-decoder = { git = "https://github.com/Nigecat/ffmpeg-video-decoder" }
```

## Usage

Simply create a `VideoDecoder` then call `next_frame`:
```rust
let decoder = VideoDecoder::new("video.mp4", false);
let first_frame = decoder.next_frame().unwrap();
```

See https://nigecat.github.io/ffmpeg-video-decoder/docs for the full docs.
