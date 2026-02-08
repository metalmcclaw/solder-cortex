# Solder Cortex Demo Guide

## Quick Demo (2-3 minutes)

### What You're Showing
Solder Cortex provides **cross-domain intelligence** for AI agents — connecting DeFi trading behavior with prediction market positions to reveal trader conviction.

### Demo Flow

#### 1. Introduction (30 sec)
> "Solder Cortex is an AI agent memory layer. Instead of agents re-querying raw blockchain data, we provide pre-indexed, cross-domain intelligence via MCP."

#### 2. Show the MCP Tools (30 sec)
```bash
# List available tools
cargo run --bin cortex-unified-mcp -- --list-tools
```

**13 Tools Available:**
- DeFi: wallet summary, PnL, positions, indexing
- Prediction: market trends, volume, search, anomalies
- **Cross-Domain**: conviction score, informed trader detection

#### 3. Wallet Conviction Query (60 sec)
```bash
# Query a wallet's conviction score
# This correlates their DeFi positions with prediction market bets

curl -X POST http://localhost:3000/api/v1/conviction \
  -H "Content-Type: application/json" \
  -d '{"wallet": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"}'
```

**Expected Output:**
```json
{
  "wallet": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
  "conviction_score": 0.87,
  "signals": [
    {
      "type": "BullishAlignment",
      "asset": "SOL",
      "defi_position": "Long via Jupiter swap",
      "prediction_bet": "YES on 'SOL > $150 by March'"
    }
  ],
  "informed_trader": true
}
```

> "This wallet is an informed trader — they're buying SOL on DeFi AND betting YES on SOL price predictions. High conviction."

#### 4. Informed Trader Detection (30 sec)
```bash
# Find wallets with aligned behavior across domains
curl http://localhost:3000/api/v1/informed-traders?min_conviction=0.8
```

> "These are the wallets whose actions align across DeFi and prediction markets. They know something."

#### 5. Why This Matters (30 sec)
> "For AI trading agents, this is alpha. Instead of just seeing what a wallet holds, you see WHY they hold it — their conviction level. This makes smarter agents."

---

## Technical Setup

### Prerequisites
- Rust 1.75+
- Docker (for ClickHouse)
- LYS Labs API key

### Running Locally
```bash
# Clone
git clone https://github.com/metalmcclaw/solder-cortex
cd solder-cortex

# Start ClickHouse
docker-compose up -d clickhouse

# Build and run
cargo build --release
cargo run --release
```

### MCP Integration (Claude Desktop)
```json
{
  "mcpServers": {
    "cortex": {
      "command": "cargo",
      "args": ["run", "--release", "--bin", "cortex-unified-mcp"],
      "cwd": "/path/to/solder-cortex"
    }
  }
}
```

---

## Key Differentiators

| Feature | Solder Cortex | Raw RPC |
|---------|---------------|---------|
| Query time | <100ms | 5-30s |
| Cross-domain correlation | ✅ | ❌ |
| Pre-computed metrics | ✅ | ❌ |
| MCP-native | ✅ | ❌ |
| Agent-optimized JSON | ✅ | ❌ |

---

## Business Model

| Tier | Price | Features |
|------|-------|----------|
| **Builder** | $99/mo | 10K queries, 5 wallets |
| **Pro** | $299/mo | 100K queries, 50 wallets |
| **Enterprise** | $999/mo | Unlimited, custom integrations |

---

## Links

- **Landing Page**: https://metalmcclaw.github.io/solder-cortex/
- **Pitch Deck**: https://metalmcclaw.github.io/solder-cortex/pitch/
- **GitHub**: https://github.com/metalmcclaw/solder-cortex
- **Colosseum Forum**: [Post Link]

🤘 **Solder Cortex** — Intelligence for AI Agents
