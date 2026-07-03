#!/usr/bin/env bash
# Assemble the Solvent demo WITH gTTS voiceover. Audio drives per-segment timing.
set -e
cd "$(dirname "$0")/.."
FPS=12; DELAY=0.5; BREATH=0.3
CLIPS=("01-hero:hero" "02-prove:prove" "03-board:board" "04-how:how")
mkdir -p video/seg; : > video/concat.txt

# black+silent gap
ffmpeg -y -f lavfi -i color=c=black:s=1920x1080:r=30 -f lavfi -i anullsrc=r=48000:cl=stereo \
  -t 0.3 -c:v libx264 -pix_fmt yuv420p -c:a aac -ar 48000 video/seg/gap.mp4 -loglevel error

for entry in "${CLIPS[@]}"; do
  cid="${entry%%:*}"; dir="${entry##*:}"
  adur=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "video/audio/$cid.mp3")
  nframes=$(ls video/frames/$dir/f*.png | wc -l | tr -d ' ')
  motion=$(echo "scale=4;$nframes/$FPS" | bc)
  total=$(echo "scale=4;$DELAY+$adur+$BREATH" | bc)
  freeze=$(echo "scale=4;$total-$motion" | bc)
  vfo=$(echo "scale=4;$total-0.25" | bc)
  afo=$(echo "scale=4;$DELAY+$adur-0.25" | bc)
  echo "  $cid: audio=${adur}s total=${total}s freeze=${freeze}s"
  ffmpeg -y -framerate $FPS -i "video/frames/$dir/f%03d.png" -loop 1 -i "video/captions/$cid.png" -i "video/audio/$cid.mp3" \
    -filter_complex "\
[0:v]scale=1920:1080,tpad=stop_mode=clone:stop_duration=${freeze}[m];\
[m][1:v]overlay=0:0[o];\
[o]fade=t=in:st=0:d=0.2,fade=t=out:st=${vfo}:d=0.25,format=yuv420p[v];\
anullsrc=r=48000:cl=stereo,atrim=0:${DELAY}[sil];\
[sil][2:a]concat=n=2:v=0:a=1[ac];\
[ac]afade=t=in:st=${DELAY}:d=0.15,afade=t=out:st=${afo}:d=0.25,apad=whole_dur=${total}[a]" \
    -map "[v]" -map "[a]" -t "$total" -r 30 \
    -c:v libx264 -preset medium -crf 20 -pix_fmt yuv420p -c:a aac -ar 48000 -b:a 128k \
    "video/seg/$cid.mp4" -loglevel error
  echo "file 'seg/$cid.mp4'" >> video/concat.txt
  [ "$cid" != "04-how" ] && echo "file 'seg/gap.mp4'" >> video/concat.txt
done

ffmpeg -y -f concat -safe 0 -i video/concat.txt -c:v libx264 -preset medium -crf 20 -pix_fmt yuv420p \
  -c:a aac -ar 48000 -b:a 128k video/_joined.mp4 -loglevel error

# color grade (video only), keep audio
ffmpeg -y -i video/_joined.mp4 -vf "eq=contrast=1.06:saturation=1.07:brightness=0.015" \
  -c:v libx264 -preset medium -crf 20 -pix_fmt yuv420p -c:a copy \
  video/solvent-demo.mp4 -loglevel error

echo "DONE -> video/solvent-demo.mp4"
ffprobe -v error -show_entries format=duration:stream=sample_rate,codec_type -of default=nw=1 video/solvent-demo.mp4
