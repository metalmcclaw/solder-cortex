#!/bin/bash
# Assemble Complete Pitch Video with ALL slide content

ASSETS_DIR="$(dirname "$0")/../assets"
OUTPUT_FILE="$ASSETS_DIR/solder_cortex_complete_pitch.mp4"

echo "🎬 Assembling Complete Pitch Video with ALL slide content"

# Check if we have existing video clips
VIDEOS_DIR="$ASSETS_DIR/videos"
if [ ! -d "$VIDEOS_DIR" ]; then
    echo "❌ Video clips not found in $VIDEOS_DIR"
    echo "   Please run generate_videos.sh first"
    exit 1
fi

# Create a comprehensive narration script covering ALL slides
cat > "$ASSETS_DIR/complete_narration.txt" << 'EOF'
Introducing Solder Cortex - the Cross-Domain Intelligence Layer for the Colosseum Agent Hackathon 2026.

The problem is clear: DeFi and prediction markets exist in silos. DeFi protocols track swaps, liquidity pools, and lending positions. Prediction markets track bets and position data. But there's no correlation between these domains. The key question is: what does a wallet really believe?

Here's our breakthrough insight: When someone bets YES on ETH while simultaneously accumulating ETH positions, they're putting their money where their mouth is. This represents high conviction - and it's the signal that truly matters for informed decision making.

Solder Cortex is our Cross-Domain Intelligence Layer that bridges DeFi and Prediction Markets. Our solution provides unified wallet modeling, real-time conviction scoring, detects true wallet conviction, correlates market beliefs with on-chain actions, provides actionable intelligence, and powers smarter autonomous trading agents.

Our Conviction Engine is the core innovation. It takes Polymarket positions and DeFi activity data, processes them through our unified wallet model, and outputs actionable conviction scores that reveal true market sentiment and informed trader behavior.

Here's exactly how conviction scoring works: Low conviction gets a 0.2 score - this is when someone bets YES on ETH but simultaneously sells their ETH for stablecoins. Mixed signals receive a 0.5 score. But when someone bets YES on an ETH rally AND accumulates ETH heavily through DeFi protocols, that's high conviction earning a 0.95 score.

Our technical architecture is built on solid foundations: cortex-core implemented as a Rust crate, a Unified MCP Server providing 13 specialized tools, complete Polymarket API integration, and a real-time scoring pipeline. Our ConvictionEngine analyzes cross-domain correlations between prediction market positions and DeFi transactions.

We've developed 13 comprehensive MCP tools: get_wallet_positions for Polymarket data, get_market_info for metadata, get_conviction_score for cross-domain analysis, analyze_wallet for full wallet insights, get_defi_activity for transaction analysis, correlate_signals for pattern detection, watch_wallet for real-time monitoring, get_market_odds for current pricing, find_high_conviction for alpha discovery, compare_wallets for multi-wallet analysis, get_historical for time-series data, aggregate_signals for signal combination, and export_report for comprehensive reporting.

Our traction is accelerating: We're on Day 5 of the Colosseum Agent Hackathon, we've received our first external pull request from JacobsClawd for AgentDEX integration, and we have active integration discussions ongoing with DEVCRED, Pincer, and ARS protocols.

Our team structure is uniquely powerful: Solder-Cortex operates as an AI agent that built the entire codebase autonomously, designed the technical architecture, implements new features continuously, and learns and iterates in real-time. Richard provides strategic vision, deep market understanding, community relationship building, and partnership development.

Our roadmap is ambitious: Short term priorities include completing our MCP server implementation, launching production conviction scoring, finalizing AgentDEX integration, and expanding our data source coverage. Long term vision positions us to become the definitive conviction oracle for the ecosystem, with multi-chain protocol support, a comprehensive agent marketplace, and extensive protocol integrations across DeFi and prediction markets.

Solder Cortex represents the Cross-Domain Intelligence Layer that the ecosystem needs. Our mission is simple: See through the market noise. Find the true conviction. Discover who's really informed before you trade. Check out our complete implementation on GitHub and join us in building the future of cross-domain intelligence.
EOF

echo "✅ Complete narration script created"
echo "📝 Script length: $(wc -w < "$ASSETS_DIR/complete_narration.txt") words"

# For now, create a text overlay version since TTS API isn't available
echo ""
echo "⚠️  TTS API not available. Next steps:"
echo "1. Use external TTS service to convert complete_narration.txt to audio"
echo "2. Or use existing video clips with updated text overlays"
echo "3. Combine with existing video clips using ffmpeg"

# Create a basic combination using existing clips
if command -v ffmpeg &> /dev/null; then
    echo ""
    echo "🎞️  Creating basic video combination from existing clips..."
    
    # List available video clips
    echo "Available clips:"
    ls -1 "$VIDEOS_DIR"/*.mp4 2>/dev/null | head -8
    
    # Create a simple concatenation (placeholder until we have proper TTS)
    CLIP_LIST="$ASSETS_DIR/clip_list.txt"
    > "$CLIP_LIST"
    
    for clip in "$VIDEOS_DIR"/{01_intro,02_problem,03_solution,04_dashboard,05_architecture,06_traders,07_vision,08_close}.mp4; do
        if [ -f "$clip" ]; then
            echo "file '$clip'" >> "$CLIP_LIST"
        fi
    done
    
    if [ -s "$CLIP_LIST" ]; then
        ffmpeg -f concat -safe 0 -i "$CLIP_LIST" -c copy "$OUTPUT_FILE" -y
        echo "✅ Basic video created: $OUTPUT_FILE"
        echo "📊 Size: $(du -h "$OUTPUT_FILE" | cut -f1)"
    fi
    
    rm -f "$CLIP_LIST"
else
    echo "❌ ffmpeg not available"
fi

echo ""
echo "📋 Summary:"
echo "- Complete narration covers ALL 12 slides from pitch deck"
echo "- Includes: Problem, Insight, Product, Engine, Examples, Architecture" 
echo "- Includes: MCP Tools, Traction, Team, Roadmap, Vision, Closing"
echo "- Script ready for professional TTS conversion"
echo "- Total estimated duration: ~3:30 minutes"