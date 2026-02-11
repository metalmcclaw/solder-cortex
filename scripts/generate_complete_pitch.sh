#!/bin/bash
# Generate Complete Pitch Video with ALL slide content

API_KEY="${GOOGLE_API_KEY}"

if [ -z "$API_KEY" ]; then
    echo "❌ Error: GOOGLE_API_KEY environment variable is not set"
    echo "Please set it with: export GOOGLE_API_KEY=your_api_key_here"
    exit 1
fi
OUTPUT_DIR="$(dirname "$0")/../assets/complete_pitch"
mkdir -p "$OUTPUT_DIR"

echo "🎬 Generating Complete Pitch Video with ALL Slides"

generate_video_clip() {
    local name="$1"
    local prompt="$2" 
    local duration="${3:-8}"
    
    echo "🎬 Generating: $name (${duration}s)"
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
    
    # Poll for completion (reduced timeout since we need many clips)
    for i in {1..30}; do
        sleep 15
        status=$(curl -s "https://generativelanguage.googleapis.com/v1beta/$operation?key=$API_KEY")
        
        if echo "$status" | grep -q '"done": true'; then
            echo "   ✅ Complete!"
            
            video_url=$(echo "$status" | grep -o '"uri": "[^"]*"' | head -1 | cut -d'"' -f4)
            if [ -n "$video_url" ]; then
                echo "   📥 Downloading: $video_url"
                curl -s -o "$OUTPUT_DIR/${name}.mp4" "$video_url"
                echo "   💾 Saved: ${name}.mp4"
            fi
            return 0
        fi
        
        if echo "$status" | grep -q '"error"'; then
            echo "   ❌ Error: $status"
            return 1
        fi
        
        echo "   ... processing ($i/30)"
    done
    
    echo "   ⏰ Timeout"
    return 1
}

# Generate all slide clips with precise timing

# Slide 1: Title (8s)
generate_video_clip "01_title" \
    "Solder Cortex logo animation with futuristic neural network connecting blockchain nodes. Text 'Cross-Domain Intelligence Layer' appears. Dark background with teal accents. Professional tech startup branding." \
    8

# Slide 2: Problem (12s)  
generate_video_clip "02_problem" \
    "Split screen visualization: Left side shows DeFi trading interface with swaps and lending. Right side shows prediction market betting interface. Red disconnect symbol between them. Data silos concept." \
    12

# Slide 3: Insight (15s)
generate_video_clip "03_insight" \
    "Animated wallet address accumulating ETH tokens while simultaneously placing YES bets on ETH rally prediction market. Green conviction meter rises. 'Money where mouth is' concept visualization." \
    15

# Slide 4: Product Intro (18s)
generate_video_clip "04_product" \
    "Cross-domain intelligence layer connecting prediction markets to DeFi protocols. Unified wallet model visualization. Real-time conviction scoring dashboard. Connecting bridges between data sources." \
    18

# Slide 5: Conviction Engine (12s)  
generate_video_clip "05_engine" \
    "Technical diagram: Polymarket data and DeFi activity flowing into unified wallet model, processed by conviction engine, outputting actionable conviction scores. Data pipeline visualization." \
    12

# Slide 6: Conviction Examples (15s)
generate_video_clip "06_examples" \
    "Three conviction score examples: Low conviction (0.2) shows conflicting signals in red. Mixed signals (0.5) in yellow. High conviction (0.95) shows aligned betting and accumulation in green." \
    15

# Slide 7: Architecture (12s)
generate_video_clip "07_architecture" \
    "Rust codebase architecture diagram: cortex-core crate, MCP server with 13 tools, Polymarket API integration, real-time pipeline. Blueprint style technical visualization." \
    12

# Slide 8: MCP Tools (10s)
generate_video_clip "08_mcp_tools" \
    "Grid layout of 13 MCP tools: get_wallet_positions, market_info, conviction_score, analyze_wallet, defi_activity, correlate_signals, watch_wallet, market_odds, find_high_conviction, compare_wallets, historical_data, aggregate_signals, export_report." \
    10

# Slide 9: Traction (10s)
generate_video_clip "09_traction" \
    "Colosseum Agent Hackathon Day 5 progress. AgentDEX integration PR from JacobsClawd. Partnership discussions with DEVCRED, Pincer, ARS logos. Community engagement metrics." \
    10

# Slide 10: Team (12s)
generate_video_clip "10_team" \
    "Split screen: Left shows AI agent (Solder-Cortex) coding autonomously with flowing code streams. Right shows human (Richard) providing strategy and vision. Collaboration visualization." \
    12

# Slide 11: Roadmap (10s)
generate_video_clip "11_roadmap" \
    "Timeline showing short-term: MCP completion, conviction scoring launch, AgentDEX integration. Long-term: conviction oracle, multi-chain support, agent marketplace, protocol integrations." \
    10

# Slide 12: Closing (8s)
generate_video_clip "12_closing" \
    "Solder Cortex logo with rock hand emoji 🤘. Text 'See through the noise. Find the conviction.' GitHub link appears. Professional closing with call-to-action." \
    8

echo ""
echo "=== Video Clip Generation Complete ==="
echo "Generated clips in: $OUTPUT_DIR"
ls -la "$OUTPUT_DIR"

# Note: Audio will be generated separately and combined
echo ""
echo "⚠️  Next steps:"
echo "1. Generate TTS audio for each segment"
echo "2. Combine video clips with matching audio"
echo "3. Assemble final video with transitions"