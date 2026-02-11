#!/bin/bash
# Solder Cortex Video Generation Script using Google Veo 3

API_KEY="${GOOGLE_API_KEY}"

if [ -z "$API_KEY" ]; then
    echo "❌ Error: GOOGLE_API_KEY environment variable is not set"
    echo "Please set it with: export GOOGLE_API_KEY=your_api_key_here"
    exit 1
fi
OUTPUT_DIR="$(dirname "$0")/../assets/videos"
mkdir -p "$OUTPUT_DIR"

generate_video() {
    local name="$1"
    local prompt="$2"
    local duration="${3:-8}"
    
    echo "🎬 Generating: $name"
    echo "   Prompt: ${prompt:0:80}..."
    
    response=$(curl -s "https://generativelanguage.googleapis.com/v1beta/models/veo-3.0-generate-001:predictLongRunning?key=$API_KEY" \
        -H "Content-Type: application/json" \
        -d "{
            \"instances\": [{
                \"prompt\": \"$prompt\"
            }],
            \"parameters\": {
                \"aspectRatio\": \"16:9\",
                \"durationSeconds\": $duration
            }
        }")
    
    operation=$(echo "$response" | grep -o '"name": "[^"]*"' | head -1 | cut -d'"' -f4)
    
    if [ -z "$operation" ]; then
        echo "   ❌ Failed to start: $response"
        return 1
    fi
    
    echo "   ⏳ Operation: $operation"
    echo "$operation" >> "$OUTPUT_DIR/operations.txt"
    
    # Poll for completion
    for i in {1..60}; do
        sleep 10
        status=$(curl -s "https://generativelanguage.googleapis.com/v1beta/$operation?key=$API_KEY")
        
        if echo "$status" | grep -q '"done": true'; then
            echo "   ✅ Complete!"
            
            # Extract video URL
            video_url=$(echo "$status" | grep -o '"uri": "[^"]*"' | head -1 | cut -d'"' -f4)
            if [ -n "$video_url" ]; then
                echo "   📥 Downloading: $video_url"
                curl -s -o "$OUTPUT_DIR/${name}.mp4" "$video_url"
                echo "   💾 Saved: $OUTPUT_DIR/${name}.mp4"
            else
                echo "   ⚠️ No video URL in response"
                echo "$status" > "$OUTPUT_DIR/${name}_response.json"
            fi
            return 0
        fi
        
        if echo "$status" | grep -q '"error"'; then
            echo "   ❌ Error: $status"
            return 1
        fi
        
        echo "   ... still processing ($i/60)"
    done
    
    echo "   ⏰ Timeout waiting for video"
    return 1
}

# Generate video clips for Solder Cortex pitch

# Intro clip
generate_video "01_intro" \
    "Futuristic tech startup logo animation, glowing neural network connecting to blockchain nodes and charts. The text 'Solder Cortex' appears with cyber aesthetic. Dark background with teal and orange accents. 4K cinematic." \
    5

# Problem visualization  
generate_video "02_problem" \
    "Split screen visualization: on the left, prediction market betting interface with anonymous users. On the right, Solana blockchain transactions. A red X appears between them showing they are disconnected. Dramatic lighting." \
    6

# Solution - connection
generate_video "03_solution" \
    "The split screen merges as glowing data streams connect prediction market data to blockchain wallets. A conviction score meter rises from 0 to 1. Green checkmarks appear. Futuristic data visualization." \
    8

# Dashboard demo
generate_video "04_dashboard" \
    "Sleek modern web dashboard showing real-time cryptocurrency analytics. Wallet addresses scroll by, conviction scores update, green profit numbers flash. Dark mode interface with charts and graphs. Professional fintech UI." \
    8

# Architecture diagram
generate_video "05_architecture" \
    "Technical architecture diagram animating into existence: Rust server connecting to ClickHouse and PostgreSQL databases, WebSocket connections flowing, MCP protocol icons. Blueprint style, engineering aesthetic." \
    6

# Informed traders
generate_video "06_informed_traders" \
    "3D visualization of a network graph. Some wallet nodes glow brighter than others - these are the informed traders. Data flows between them. Heat map overlay shows conviction levels. Dark mode, matrix style." \
    8

# Vision - future
generate_video "07_vision" \
    "Futuristic cityscape with AI agents represented as light beings, each one connected to the Solana blockchain. Cross-domain intelligence flows between them. Text 'The Intelligence Layer for AI Agents' fades in. Epic cinematic." \
    8

# Closing logo
generate_video "08_close" \
    "Solder Cortex logo animation with metal/industrial aesthetic. The rock hand emoji 🤘 appears. Sparks and welding effects. Text 'Know who's informed before you trade' appears below. Professional end card." \
    5

echo ""
echo "=== Video Generation Complete ==="
echo "Videos saved to: $OUTPUT_DIR"
ls -la "$OUTPUT_DIR"
