#!/bin/bash

rm -r docs
cargo doc --no-deps --open "$@"
mv -v target/doc docs
echo "<meta http-equiv=\"refresh\" content=\"0; url=ffmpeg_video_decoder\">" > docs/index.html
