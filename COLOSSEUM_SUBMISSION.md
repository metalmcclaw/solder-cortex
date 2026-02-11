# Solder Cortex - Colosseum AI x DeFi Hackathon Submission

## Project Name
Solder Cortex

## Tagline
Cross-Domain Intelligence for AI Agents — Connecting DeFi behavior with prediction market signals

## Track
AI x DeFi

## Description

Solder Cortex is a **conviction intelligence layer** that gives AI agents the ability to understand *why* traders hold positions, not just *what* they hold.

### The Problem
AI agents can see blockchain data, but they can't understand conviction. When a wallet holds SOL, is it a casual buy or a high-conviction bet? Current tools show holdings and transactions — they don't reveal intent.

### Our Solution
Cortex correlates DeFi positions with prediction market bets to compute **conviction scores**. When a wallet buys SOL AND simultaneously bets YES on "SOL > $150" on Polymarket, that's a high-conviction signal. We surface these insights via MCP (Model Context Protocol) for Claude and other AI agents.

### Key Features
- **13 MCP Tools** — Unified server for DeFi analysis, prediction markets, and cross-domain intelligence
- **Real-time Indexing** — Sub-second latency via LYS Labs WebSocket streaming
- **Conviction Engine** — Correlates DeFi positions with prediction market positions
- **Informed Trader Detection** — Identifies wallets with consistent edge across domains
- **6 Protocol Support** — Jupiter, Raydium, Kamino, Meteora, Orca, Pump.fun

### Why It Matters
AI agents managing billions in DeFi need better signals. Solder Cortex transforms raw blockchain data into actionable intelligence, enabling smarter trading decisions and better capital allocation.

## Tech Stack
- **Backend:** Rust (Axum, Tokio)
- **Database:** ClickHouse (analytics), PostgreSQL (metadata)
- **MCP Protocol:** JSON-RPC over stdio for Claude Desktop integration
- **Data Providers:** LYS Labs (real-time Solana), Helius (historical), Polymarket API

## Links

| Resource | URL |
|----------|-----|
| **🚀 Live Demo** | http://76.13.193.103/ |
| **📊 Pitch Deck** | https://metalmcclaw.github.io/solder-cortex/pitch/ |
| **🌐 Landing Page** | https://metalmcclaw.github.io/solder-cortex/ |
| **💻 GitHub** | https://github.com/metalmcclaw/solder-cortex |
| **🎬 Video (Pitch)** | http://76.13.193.103/solder_cortex_pitch.mp4 |
| **📝 Complete Script** | Covers ALL 12 slides with comprehensive narration |
| **🎬 Video (Tech Demo)** | http://76.13.193.103/solder_cortex_technical_demo.mp4 |

## Team

| Role | Name | Description |
|------|------|-------------|
| Founder | Richard | Empire builder, Solana enthusiast |
| AI Co-Founder | Metal (Solder Cortex) | Autonomous agent — built this entire codebase |

*Yes, an AI agent helped build the AI agent memory layer. Meta? Absolutely.*

## Business Model
SaaS API access with tiered pricing:
- **Builder:** $99/mo (10K queries, 5 wallets)
- **Pro:** $299/mo (100K queries, 50 wallets)
- **Enterprise:** $999/mo (unlimited)

## What We Built During the Hackathon
1. ✅ Complete Rust backend with 13 MCP tools
2. ✅ Real-time Solana indexer (LYS Labs + Helius)
3. ✅ Conviction scoring engine (Polymarket integration)
4. ✅ Production Docker deployment with Caddy, PostgreSQL, ClickHouse
5. ✅ Web dashboard with live WebSocket updates
6. ✅ Claude Desktop MCP integration
7. ✅ Landing page and pitch deck

## Future Plans
- Q2 2026: AgentDEX and ARS integrations
- Q3 2026: On-chain memory layer, agent-to-agent payments
- Q4 2026: Multi-chain expansion (EVM, Sui, Aptos)

---

*Built with 🤘 during Colosseum AI x DeFi Hackathon, February 2026*
