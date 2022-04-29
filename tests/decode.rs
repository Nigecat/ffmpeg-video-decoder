use ffmpeg_video_decoder::{VideoDecoder, VideoSource};
use std::path::PathBuf;

fn run_decode_test(source: VideoSource) {
    let mut decoder = VideoDecoder::new(source, false).unwrap();

    // Check video dimensions are correct
    assert_eq!(decoder.dimensions().height(), 1080);
    assert_eq!(decoder.dimensions().width(), 1920);

    // Check video framerate is correct
    assert_eq!(decoder.framerate(), 30.0);

    let mut max = 0;
    while let Some(frame) = decoder.next_frame().unwrap() {
        max = frame.index();
        assert_eq!(frame.dimensions(), decoder.dimensions());
    }

    assert_eq!(max, 899); // test video has 901 frames
}

#[test]
fn file() {
    let source = PathBuf::from("test.mp4");
    run_decode_test(source.into());
}

#[test]
fn unicode_file() {
    let source = PathBuf::from("テスト.mp4");
    run_decode_test(source.into());
}

#[test]
fn memory() {
    let source = include_bytes!("../test.mp4").to_vec();
    run_decode_test(source.into());
}
