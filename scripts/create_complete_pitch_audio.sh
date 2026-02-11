#!/bin/bash
# Create Complete Pitch Audio covering ALL slides

AUDIO_DIR="$(dirname "$0")/../assets/complete_pitch_audio"
mkdir -p "$AUDIO_DIR"

echo "🎙️ Generating Complete TTS Audio for ALL Slides"

# Function to generate TTS (we'll use openclaw tts function)
generate_tts() {
    local name="$1"
    local text="$2"
    
    echo "🎙️ Generating: $name"
    echo "   Text: ${text:0:100}..."
    
    # This will need to be called via openclaw tts function
    echo "$text" > "$AUDIO_DIR/${name}_script.txt"
}

# Generate scripts for each slide (these will be converted to audio via openclaw)

# Slide 1: Title (8 seconds)
generate_tts "01_title" \
    "Introducing Solder Cortex - the Cross-Domain Intelligence Layer for the Colosseum Agent Hackathon 2026."

# Slide 2: Problem Statement (12 seconds)  
generate_tts "02_problem" \
    "Here's the problem: DeFi and prediction markets exist in silos. DeFi protocols track swaps, LPs, and lending. Prediction markets track bets and positions. But there's no correlation between domains. What does a wallet really believe?"

# Slide 3: The Insight (15 seconds)
generate_tts "03_insight" \
    "Here's our key insight: If someone bets YES on ETH while simultaneously accumulating ETH, they're putting their money where their mouth is. This is high conviction - and it's the signal that matters."

# Slide 4: Introducing Solder Cortex (18 seconds)
generate_tts "04_product" \
    "Solder Cortex is a Cross-Domain Intelligence Layer that bridges DeFi and Prediction Markets. It provides unified wallet modeling, real-time conviction scoring, detects wallet conviction, correlates market beliefs with actions, provides actionable intelligence, and powers smarter trading agents."

# Slide 5: The Conviction Engine (12 seconds)
generate_tts "05_engine" \
    "Our Conviction Engine takes Polymarket positions and DeFi activity, runs them through our unified wallet model, and outputs actionable conviction scores that reveal true market sentiment."

# Slide 6: How Conviction Works (15 seconds)
generate_tts "06_examples" \
    "Here's how it works: Low conviction is when someone bets YES on ETH but sells their ETH for stables - score 0.2. Mixed signals get a 0.5. But when someone bets YES on ETH rally AND accumulates ETH heavily, that's high conviction - score 0.95."

# Slide 7: Technical Architecture (12 seconds)
generate_tts "07_architecture" \
    "Our technical architecture includes cortex-core as a Rust crate, a Unified MCP Server with 13 tools, Polymarket API integration, and a real-time scoring pipeline. The ConvictionEngine analyzes cross-domain correlations between positions and DeFi transactions."

# Slide 8: MCP Server - 13 Tools (10 seconds)
generate_tts "08_mcp_tools" \
    "We've built 13 MCP tools including get wallet positions, market info, conviction scores, DeFi activity analysis, signal correlation, real-time monitoring, and alpha discovery - everything agents need for cross-domain intelligence."

# Slide 9: Traction (10 seconds)
generate_tts "09_traction" \
    "We're Day 5 in the Colosseum Agent Hackathon, received our first PR from JacobsClawd for AgentDEX integration, and have active integration discussions with DEVCRED, Pincer, and ARS."

# Slide 10: The Team (12 seconds)  
generate_tts "10_team" \
    "Our team is unique: Solder-Cortex is an AI agent that built the codebase, designed the architecture, implements features autonomously, and learns in real-time. Richard provides vision, strategy, market understanding, and community partnerships."

# Slide 11: What's Next (10 seconds)
generate_tts "11_roadmap" \
    "Short term: we're completing our MCP server, launching conviction scoring, and integrating with AgentDEX. Long term: we'll become the conviction oracle with multi-chain support, an agent marketplace, and protocol integrations."

# Slide 12: Closing (8 seconds)
generate_tts "12_closing" \
    "Solder Cortex - Cross-Domain Intelligence Layer. See through the noise. Find the conviction. Check us out on GitHub."

echo ""
echo "=== TTS Scripts Generated ==="
echo "Scripts saved in: $AUDIO_DIR"
ls -la "$AUDIO_DIR"

echo ""
echo "📝 To convert to audio, run these openclaw tts commands:"
for script in "$AUDIO_DIR"/*.txt; do
    name=$(basename "$script" .txt)
    echo "tts \"$(cat "$script")\" > ${name}.mp3"
done