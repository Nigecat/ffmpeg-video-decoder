# ffmpeg-video-decoder
A simple (safe) wrapper around ffmpeg-sys to provide a basic video decoder

## Usage

Simply create a `VideoDecoder` then call `next_frame`:
```rust
let decoder = VideoDecoder::new("video.mp4", false);
let first_frame = decoder.next_frame().unwrap();
```
