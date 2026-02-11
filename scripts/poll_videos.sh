#!/bin/bash
# Poll and download completed Veo videos

API_KEY="${GOOGLE_API_KEY}"

if [ -z "$API_KEY" ]; then
    echo "❌ Error: GOOGLE_API_KEY environment variable is not set"
    echo "Please set it with: export GOOGLE_API_KEY=your_api_key_here"
    exit 1
fi
OUTPUT_DIR="$(dirname "$0")/../assets/videos"
PENDING_FILE="$OUTPUT_DIR/pending_operations.txt"

poll_and_download() {
    while IFS='|' read -r name operation; do
        [ -z "$operation" ] && continue
        
        # Skip if already downloaded
        [ -f "$OUTPUT_DIR/${name}.mp4" ] && continue
        
        echo "Checking: $name"
        status=$(curl -s "https://generativelanguage.googleapis.com/v1beta/$operation?key=$API_KEY")
        
        if echo "$status" | grep -q '"done": true'; then
            video_uri=$(echo "$status" | grep -o '"uri": "[^"]*"' | head -1 | cut -d'"' -f4)
            
            if [ -n "$video_uri" ]; then
                echo "  ✅ Downloading $name..."
                curl -sL "${video_uri}&key=$API_KEY" -o "$OUTPUT_DIR/${name}.mp4"
                
                size=$(ls -la "$OUTPUT_DIR/${name}.mp4" | awk '{print $5}')
                echo "  💾 Saved: ${name}.mp4 ($size bytes)"
            else
                echo "  ⚠️ No video URI found"
                echo "$status" > "$OUTPUT_DIR/${name}_error.json"
            fi
        elif echo "$status" | grep -q '"error"'; then
            echo "  ❌ Error generating $name"
            echo "$status" > "$OUTPUT_DIR/${name}_error.json"
        else
            echo "  ⏳ Still processing..."
        fi
    done < "$PENDING_FILE"
}

echo "=== Polling Veo Video Operations ==="
echo "Output: $OUTPUT_DIR"
echo ""

poll_and_download

echo ""
echo "=== Current Video Files ==="
ls -la "$OUTPUT_DIR"/*.mp4 2>/dev/null || echo "No videos yet"
