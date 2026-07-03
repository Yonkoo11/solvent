#!/usr/bin/env bash
# Assemble the Solvent captions-only motion demo. No voiceover (TTS unavailable).
set -e
cd "$(dirname "$0")/.."
FPS=12
# clip -> source-frames-dir : target-seconds
CLIPS=("01-hero:hero:6.5" "02-prove:prove:11" "03-board:board:10" "04-how:how:10")
mkdir -p video/seg
: > video/concat.txt

for entry in "${CLIPS[@]}"; do
  cid="${entry%%:*}"; rest="${entry#*:}"; dir="${rest%%:*}"; target="${rest##*:}"
  nframes=$(ls video/frames/$dir/f*.png | wc -l | tr -d ' ')
  motion=$(echo "scale=4;$nframes/$FPS" | bc)
  freeze=$(echo "scale=4;$target-$motion" | bc)
  fo=$(echo "scale=4;$target-0.25" | bc)
  echo "  $cid: $nframes frames motion=${motion}s freeze=${freeze}s target=${target}s"
  ffmpeg -y -framerate $FPS -i "video/frames/$dir/f%03d.png" -loop 1 -i "video/captions/$cid.png" \
    -filter_complex "[0:v]scale=1920:1080,tpad=stop_mode=clone:stop_duration=${freeze}[m];\
[m][1:v]overlay=0:0[o];\
[o]fade=t=in:st=0:d=0.25,fade=t=out:st=${fo}:d=0.25,format=yuv420p[v]" \
    -map "[v]" -t "$target" -r 30 -c:v libx264 -preset medium -crf 20 -pix_fmt yuv420p \
    "video/seg/$cid.mp4" -loglevel error
  echo "file 'seg/$cid.mp4'" >> video/concat.txt
  # 0.3s black gap between clips
  if [ "$cid" != "04-how" ]; then
    gap="video/seg/gap-$cid.mp4"
    ffmpeg -y -f lavfi -i color=c=black:s=1920x1080:r=30 -t 0.3 -c:v libx264 -pix_fmt yuv420p "$gap" -loglevel error
    echo "file 'seg/$(basename $gap)'" >> video/concat.txt
  fi
done

# concat (re-encode to avoid drift)
ffmpeg -y -f concat -safe 0 -i video/concat.txt -c:v libx264 -preset medium -crf 20 -pix_fmt yuv420p video/_joined.mp4 -loglevel error

# color grade + silent 48k audio track
ffmpeg -y -i video/_joined.mp4 -f lavfi -i anullsrc=r=48000:cl=stereo \
  -vf "eq=contrast=1.06:saturation=1.07:brightness=0.015" \
  -map 0:v -map 1:a -shortest \
  -c:v libx264 -preset medium -crf 20 -pix_fmt yuv420p -c:a aac -ar 48000 -b:a 128k \
  video/solvent-demo.mp4 -loglevel error

echo "DONE -> video/solvent-demo.mp4"
ffprobe -v error -show_entries format=duration:stream=sample_rate,codec_type -of default=nw=1 video/solvent-demo.mp4
