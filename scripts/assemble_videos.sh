#!/bin/bash
# Assemble final hackathon videos from Veo clips + TTS voiceovers

cd "$(dirname "$0")/../assets"

echo "=== Assembling Pitch Video ==="

# Create a text file listing clips for concat (we'll loop clips to extend duration)
cat > pitch_clips.txt << EOF
file 'videos/01_intro.mp4'
file 'videos/01_intro.mp4'
file 'videos/02_problem.mp4'
file 'videos/02_problem.mp4'
file 'videos/02_problem.mp4'
file 'videos/02_problem.mp4'
file 'videos/03_solution.mp4'
file 'videos/03_solution.mp4'
file 'videos/03_solution.mp4'
file 'videos/03_solution.mp4'
file 'videos/03_solution.mp4'
file 'videos/03_solution.mp4'
file 'videos/04_dashboard.mp4'
file 'videos/04_dashboard.mp4'
file 'videos/04_dashboard.mp4'
file 'videos/04_dashboard.mp4'
EOF

# Concatenate video clips (no audio)
ffmpeg -y -f concat -safe 0 -i pitch_clips.txt -c:v libx264 -preset fast -crf 23 -an pitch_video_only.mp4

# Add voiceover
ffmpeg -y -i pitch_video_only.mp4 -i pitch_voiceover.mp3 \
    -c:v copy -c:a aac -b:a 192k \
    -map 0:v:0 -map 1:a:0 \
    -shortest \
    solder_cortex_pitch.mp4

echo "✅ Pitch video: solder_cortex_pitch.mp4"
ls -la solder_cortex_pitch.mp4

echo ""
echo "=== Assembling Technical Demo Video ==="

# Technical demo clips
cat > tech_clips.txt << EOF
file 'videos/01_intro.mp4'
file 'videos/05_architecture.mp4'
file 'videos/05_architecture.mp4'
file 'videos/05_architecture.mp4'
file 'videos/05_architecture.mp4'
file 'videos/04_dashboard.mp4'
file 'videos/04_dashboard.mp4'
file 'videos/04_dashboard.mp4'
file 'videos/04_dashboard.mp4'
file 'videos/04_dashboard.mp4'
file 'videos/06_traders.mp4'
file 'videos/06_traders.mp4'
file 'videos/06_traders.mp4'
file 'videos/06_traders.mp4'
file 'videos/06_traders.mp4'
file 'videos/06_traders.mp4'
file 'videos/08_close.mp4'
file 'videos/08_close.mp4'
EOF

# Concatenate
ffmpeg -y -f concat -safe 0 -i tech_clips.txt -c:v libx264 -preset fast -crf 23 -an tech_video_only.mp4

# Add voiceover
ffmpeg -y -i tech_video_only.mp4 -i technical_demo_voiceover.mp3 \
    -c:v copy -c:a aac -b:a 192k \
    -map 0:v:0 -map 1:a:0 \
    -shortest \
    solder_cortex_technical_demo.mp4

echo "✅ Technical demo: solder_cortex_technical_demo.mp4"
ls -la solder_cortex_technical_demo.mp4

# Cleanup temp files
rm -f pitch_clips.txt tech_clips.txt pitch_video_only.mp4 tech_video_only.mp4

echo ""
echo "=== Final Videos ==="
ls -la solder_cortex_*.mp4

echo ""
echo "=== Video Info ==="
for f in solder_cortex_*.mp4; do
    echo "--- $f ---"
    ffprobe -v error -show_entries format=duration,size -of default=noprint_wrappers=1 "$f"
done
