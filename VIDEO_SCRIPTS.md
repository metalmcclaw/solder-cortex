# Solder Cortex - Video Scripts

## 1. Pitch Video Script (3 minutes max)

### Intro (15 sec)
"Hey, I'm building Solder Cortex - a cross-domain intelligence layer that connects DeFi analytics with prediction markets on Solana."

### Problem (30 sec)
"Here's the problem: Prediction markets tell you what the crowd thinks. DeFi shows you what people actually do with their money. But these two worlds are completely disconnected.

When someone bets YES on 'SOL will hit $200' - is that just noise, or do they actually have skin in the game on-chain? Right now, there's no way to know."

### Solution (45 sec)
"Solder Cortex bridges this gap. We built an MCP server that correlates prediction market bets with on-chain DeFi positions.

Our conviction scoring engine works like this: If a wallet bets bullish on Polymarket AND holds leveraged longs on Jupiter, they're not just talking - they're ALL IN. We surface that signal.

The result? A conviction score from 0 to 1 that tells you how aligned someone's bets are with their actual on-chain behavior."

### Demo Highlight (30 sec)
"Let me show you quickly - we have 9 MCP tools live right now. You can query any wallet's conviction score, detect informed traders in any prediction market, and get real-time indexing of wallet activity.

This is running live at our demo URL with real Solana data from Helius and LysLabs."

### Validation (20 sec)
"We've already gotten positive feedback from the agent community on Moltbook. The cross-domain intelligence concept resonates because it's solving a real blind spot - traders want to know who's informed before they trade."

### Vision (30 sec)
"Our vision is to become the intelligence layer for AI agents trading on Solana. Every agent should be able to ask: 'Is this trader informed? What's their conviction?' 

We're building the memory that makes agents smarter."

### Close (10 sec)
"Solder Cortex. Know who's informed before you trade. Check out our live demo and let's build the future of cross-domain intelligence together."

---

## 2. Technical Demo Script (2-3 minutes)

### Intro (10 sec)
"Let me walk you through how Solder Cortex works under the hood."

### Architecture Overview (30 sec)
"We built a unified MCP server in Rust that exposes 9 tools. The architecture is:
- A Rust API server handling DeFi data ingestion
- ClickHouse for time-series analytics
- PostgreSQL for relational data
- A WebSocket bridge for the web dashboard
- All containerized with Docker Compose"

### Solana Integration (40 sec)
"For Solana integration, we use two data sources:
- Helius API for historical transaction data
- LysLabs WebSocket for real-time streaming

When you call `cortex_start_indexing`, we fetch historical transactions from Helius, parse DeFi interactions, and store them in ClickHouse. Then LysLabs keeps us updated in real-time.

Here's it running - you can see we're indexing hundreds of transactions per second."

### Conviction Engine (45 sec)
"The core innovation is our conviction scoring algorithm.

When you call `cortex_get_wallet_conviction`, we:
1. Fetch DeFi positions from our indexed data
2. Query Polymarket for prediction market bets
3. Correlate them using token/topic matching
4. Calculate a conviction score based on alignment

If someone's DeFi position and prediction bet point the same direction, conviction goes up. Cross-domain activity gives bonus weight because it's a stronger signal."

### Live Demo (30 sec)
"Let me show this live. I'll call the health endpoint... API is up. Now let's check a wallet's conviction... You can see the score, the signal classification, and whether they're an informed trader.

The `cortex_detect_informed_traders` tool does this at scale - finding all wallets with cross-domain activity in a given market."

### Close (15 sec)
"Everything's open source on GitHub. The MCP binary works with Claude Desktop, OpenClaw, or any MCP client. Thanks for watching."

---

## Recording Notes

### For Pitch Video:
- Use slides or website as visual background
- Voiceover with ElevenLabs TTS
- Show quick demo clip in the middle
- Keep energy high, this is a pitch

### For Technical Demo:
- Screen record the terminal and dashboard
- Show actual commands running
- Can use more technical tone
- Show real data flowing

### Tools Needed:
- ffmpeg for screen recording
- ElevenLabs for TTS (API key in OpenClaw env)
- Browser showing http://76.13.193.103/
- Terminal showing docker logs and wscat commands
