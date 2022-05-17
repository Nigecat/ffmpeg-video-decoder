#!/bin/bash

rm -rv docs
cargo +nightly doc --no-deps --features image "$@" 
mv -v target/doc docs
mv -v docs/ffmpeg_video_decoder docs/docs
echo "<meta http-equiv=\"refresh\" content=\"0; url=docs\">" > docs/index.html
