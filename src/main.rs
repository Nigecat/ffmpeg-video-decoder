use ffmpeg_video_decoder::VideoDecoder;

fn main() {
    let source: Vec<u8> = include_bytes!("../test.mp4").to_vec();
    // let source = std::path::PathBuf::from("test.mp4");
    // let source = std::path::PathBuf::from("テスト.mp4");
    let mut decoder = VideoDecoder::new(source, true).unwrap();

    // let frame = decoder.next_frame().unwrap().unwrap().into_image();
    // frame.save("frame1.png").unwrap();
    // let frame = decoder.next_frame().unwrap().unwrap().into_image();
    // frame.save("frame2.png").unwrap();

    while let Some(frame) = decoder.next_frame().unwrap() {
        println!("Got frame: {} w/{:?}", frame.index(), frame.dimensions());
    }
}
