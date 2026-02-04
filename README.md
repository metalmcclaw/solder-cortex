# Solder Cortex

**AI Agent Memory Layer** - Pre-indexed financial data for AI agents via MCP (Model Context Protocol).

Cortex provides AI agents with fast access to DeFi and prediction market data through MCP servers that connect directly to ClickHouse. It solves the problem that AI agents cannot "remember" financial history without re-querying raw data sources, which is slow and expensive.

## Features

- **MCP-native** - Two MCP servers for Claude and other AI agents
- **Single API call** to fetch comprehensive wallet state (PnL, positions, risk)
- **Pre-computed metrics** - no need to decode logs or calculate PnL
- **Agent-optimized JSON** - clean responses designed for LLM consumption
- **Sub-second latency** - ClickHouse-powered analytics
- **Real-time data** - powered by LYS Labs WebSocket streaming

## MCP Servers

Cortex provides two MCP servers for AI agent integration:

| Server | Purpose | Tools |
|--------|---------|-------|
| `cortex-mcp` | Solana DeFi wallet data | 7 tools |
| `cortex-prediction-mcp` | Prediction market analytics | 4 tools |

### Solana DeFi MCP (`cortex-mcp`)

Tools for Solana wallet analysis and indexing:

| Tool | Description |
|------|-------------|
| `cortex_health` | Check Cortex service health |
| `cortex_get_wallet_summary` | Get wallet overview (value, PnL, risk, protocols) |
| `cortex_get_wallet_pnl` | Get PnL breakdown by protocol (24h/7d/30d/all) |
| `cortex_get_wallet_positions` | Get current open positions across protocols |
| `cortex_start_indexing` | Start continuous indexing for a wallet |
| `cortex_stop_indexing` | Stop indexing for a wallet |
| `cortex_list_subscriptions` | List all wallets being indexed |

### Prediction Market MCP (`cortex-prediction-mcp`)

Tools for prediction market analysis:

| Tool | Description |
|------|-------------|
| `get_market_trend` | OHLCV data with configurable intervals (1m to 7d) |
| `get_volume_profile` | 24h/7d volume, trade counts, liquidity depth |
| `search_market_memory` | Full-text search across market titles/descriptions |
| `detect_anomalies` | Find price spikes >N std devs from moving average |

## Supported Protocols

### Solana DeFi
- **Jupiter** - DEX aggregator (swaps)
- **Raydium** - AMM/CLMM DEX (swaps, LP)
- **Kamino** - Lending protocol (supply, borrow)
- **Meteora** - DLMM DEX (swaps)
- **Orca** - Whirlpool DEX (swaps)
- **Pump.fun** - Token launchpad (swaps)

### Prediction Markets
- **Polymarket** - Prices, trades, order book
- **Kalshi** - Prices, trades, order book

## Quick Start

### Prerequisites

- Rust 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Docker & Docker Compose (for ClickHouse)
- LYS Labs API key (get one at https://dev.lyslabs.ai/)

### Development Setup

```bash
# Clone and enter directory
cd cortex

# Copy environment template
cp .env.example .env

# Edit .env and add your LYS Labs API key
# CORTEX_LYSLABS_API_KEY=your-api-key-here

# Start ClickHouse
docker-compose up -d clickhouse

# Build and run
cargo build --release
cargo run
```

### Environment Configuration

Create a `.env` file with the following variables (note: use double underscores `__` as separators):

```bash
# Server Configuration
CORTEX__SERVER__HOST=0.0.0.0
CORTEX__SERVER__PORT=3000

# ClickHouse Database Configuration
CORTEX__DATABASE__URL=http://localhost:8123
CORTEX__DATABASE__DATABASE=cortex
CORTEX__DATABASE__USER=default
CORTEX__DATABASE__PASSWORD=clickhouse

# LYS Labs API Configuration
# Get your API key from https://dev.lyslabs.ai/
CORTEX__LYSLABS__API_KEY=your-lyslabs-api-key-here
CORTEX__LYSLABS__WS_URL=wss://solana-mainnet-api-vip.lyslabs.ai/v1/

# Logging
RUST_LOG=cortex=debug,tower_http=debug
```

### Using Docker

```bash
# Set environment variable
export LYSLABS_API_KEY=your_api_key_here

# Start everything
docker-compose up -d

# Check logs
docker-compose logs -f cortex
```

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check |
| `GET /api/v1/user/{wallet}/summary` | Wallet overview (PnL, risk, protocols) |
| `GET /api/v1/user/{wallet}/pnl?window=7d` | PnL breakdown by protocol |
| `GET /api/v1/user/{wallet}/positions` | Current open positions |
| `POST /api/v1/index` | Trigger wallet indexing |

See [API.md](./API.md) for full documentation.

## Example Usage

```bash
# Index a wallet (streams real-time transactions for the wallet)
curl -X POST http://localhost:3000/api/v1/index \
  -H "Content-Type: application/json" \
  -d '{"wallet": "95n9a8yd6aZzKGMtbWSjqbijZ1u99z1GQF79HkbCvtwN"}'

# Get wallet summary
curl http://localhost:3000/api/v1/user/95n9a8yd6aZzKGMtbWSjqbijZ1u99z1GQF79HkbCvtwN/summary
```

**Response:**
```json
{
  "wallet": "95n9a8yd6aZzKGMtbWSjqbijZ1u99z1GQF79HkbCvtwN",
  "total_value_usd": 15420.50,
  "pnl": {
    "realized_24h": 120.30,
    "realized_7d": 540.00,
    "unrealized": 890.25
  },
  "risk": {
    "score": 45,
    "largest_position_pct": 0.35,
    "protocol_count": 3
  },
  "protocols": ["jupiter", "raydium", "kamino"]
}
```

## Claude Code Integration

Add the MCP servers to Claude Code for AI-assisted DeFi and prediction market analysis.

### Solana DeFi MCP

Requires `cortex-server` to be running.

```bash
# Build the MCP server
cargo build -p cortex-mcp --release

# Add to Claude Code
claude mcp add -s user cortex \
  $(pwd)/target/release/cortex-mcp \
  -e CORTEX_API_URL=http://localhost:3000
```

### Prediction Market MCP

Connects directly to ClickHouse (no cortex-server required).

```bash
# Build the MCP server
cargo build -p cortex-prediction-mcp --release

# Add to Claude Code
claude mcp add -s user cortex-prediction \
  $(pwd)/target/release/cortex-prediction-mcp \
  -e CORTEX_PREDICTION__DATABASE__URL=http://localhost:8123 \
  -e CORTEX_PREDICTION__DATABASE__DATABASE=cortex \
  -e CORTEX_PREDICTION__DATABASE__USER=default \
  -e CORTEX_PREDICTION__DATABASE__PASSWORD=clickhouse \
  -e CORTEX_PREDICTION__CACHE__MAX_CAPACITY=1000 \
  -e CORTEX_PREDICTION__CACHE__TTL_SECONDS=60 \
  -e RUST_LOG=cortex_prediction_mcp=info
```

### Verify Installation

```bash
# List configured MCP servers
claude mcp list

# Test in Claude Code
/mcp
```

### Manual Testing

```bash
# Test Prediction MCP initialization
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  CORTEX_PREDICTION__DATABASE__URL=http://localhost:8123 \
  CORTEX_PREDICTION__DATABASE__DATABASE=cortex \
  ./target/release/cortex-prediction-mcp
```

## Configuration

Environment variables use double underscores (`__`) as separators for nested config keys.

### Cortex Server (`cortex-server`)

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `CORTEX__SERVER__HOST` | Server bind address | `0.0.0.0` |
| `CORTEX__SERVER__PORT` | Server port | `3000` |
| `CORTEX__DATABASE__URL` | ClickHouse URL | `http://localhost:8123` |
| `CORTEX__DATABASE__DATABASE` | ClickHouse database | `cortex` |
| `CORTEX__DATABASE__USER` | ClickHouse user | `default` |
| `CORTEX__DATABASE__PASSWORD` | ClickHouse password | (empty) |
| `CORTEX__LYSLABS__API_KEY` | LYS Labs API key | (required) |
| `CORTEX__LYSLABS__WS_URL` | LYS Labs WebSocket URL | `wss://solana-mainnet-api-vip.lyslabs.ai/v1/` |
| `CORTEX__HELIUS__API_KEY` | Helius API key for historical data | (optional) |
| `RUST_LOG` | Log level | `cortex=info` |

### Solana DeFi MCP (`cortex-mcp`)

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `CORTEX_API_URL` | Cortex server URL | `http://localhost:3000` |

### Prediction Market MCP (`cortex-prediction-mcp`)

| Environment Variable | Description | Default |
|---------------------|-------------|---------|
| `CORTEX_PREDICTION__DATABASE__URL` | ClickHouse URL | `http://localhost:8123` |
| `CORTEX_PREDICTION__DATABASE__DATABASE` | ClickHouse database | `cortex` |
| `CORTEX_PREDICTION__DATABASE__USER` | ClickHouse user | `default` |
| `CORTEX_PREDICTION__DATABASE__PASSWORD` | ClickHouse password | (empty) |
| `CORTEX_PREDICTION__CACHE__MAX_CAPACITY` | Max cache entries | `1000` |
| `CORTEX_PREDICTION__CACHE__TTL_SECONDS` | Cache TTL in seconds | `60` |
| `RUST_LOG` | Log level | `cortex_prediction_mcp=info` |

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           AI AGENTS (Claude, etc.)                       │
└─────────────────────────────────────────────────────────────────────────┘
                │                                    │
                │ JSON-RPC (stdio)                   │ JSON-RPC (stdio)
                ▼                                    ▼
┌───────────────────────────┐        ┌───────────────────────────────────┐
│      cortex-mcp           │        │     cortex-prediction-mcp         │
│  (Solana DeFi Tools)      │        │  (Prediction Market Tools)        │
│                           │        │                                   │
│  • wallet_summary         │        │  • get_market_trend               │
│  • wallet_pnl             │        │  • get_volume_profile             │
│  • wallet_positions       │        │  • search_market_memory           │
│  • start/stop_indexing    │        │  • detect_anomalies               │
└───────────┬───────────────┘        └─────────────┬─────────────────────┘
            │ HTTP REST                            │ Direct ClickHouse
            ▼                                      ▼
┌───────────────────────────┐        ┌───────────────────────────────────┐
│      cortex-server        │        │         Query Cache               │
│    (REST API + Indexer)   │        │         (moka async)              │
│                           │        │                                   │
│  • GET /health            │        │  • TTL: 60s (configurable)        │
│  • GET /user/{w}/summary  │        │  • Max capacity: 1000             │
│  • POST /index            │        │                                   │
└───────────┬───────────────┘        └─────────────┬─────────────────────┘
            │                                      │
            ▼                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            CLICKHOUSE                                    │
│                                                                          │
│  Solana DeFi Tables:              Prediction Market Tables:              │
│  ├── transactions                 ├── markets                            │
│  ├── positions                    ├── market_prices                      │
│  ├── wallet_summaries             ├── market_trades                      │
│  └── token_prices                 ├── market_volume                      │
│                                   ├── market_orderbook                   │
│                                   ├── market_stats                       │
│                                   └── mv_market_volume_1h (MV)           │
└─────────────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **LYS Labs WebSocket Connection**: The indexer connects to LYS Labs' real-time WebSocket API to receive decoded Solana transaction events with sub-second latency.

2. **Transaction Filtering**: Incoming transactions are filtered by wallet address and parsed to identify DeFi operations (swaps, deposits, borrows, etc.).

3. **Protocol Detection**: The parser identifies which protocol processed the transaction (Jupiter, Raydium, Kamino, Meteora, Orca, Pump.fun) based on decoder type and program IDs.

4. **Metrics Computation**: PnL and risk metrics are computed from the parsed transaction data.

5. **Storage**: Processed transactions and computed metrics are stored in ClickHouse for fast analytical queries.

6. **API Serving**: The Axum-based REST API serves pre-computed wallet data to AI agents.

## Project Structure

```
cortex/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── cortex-server/            # Main API server + indexer
│   │   └── src/
│   │       ├── main.rs           # Entry point
│   │       ├── config.rs         # Configuration
│   │       ├── error.rs          # Error types
│   │       ├── types.rs          # Domain types
│   │       ├── api/              # HTTP handlers
│   │       ├── db/               # ClickHouse queries
│   │       ├── indexer/          # LYS Labs + Helius clients
│   │       │   └── protocols/    # Protocol parsers
│   │       └── metrics/          # PnL and risk calculations
│   │
│   ├── cortex-mcp/               # MCP bridge to cortex-server
│   │   └── src/main.rs           # Pure JSON-RPC implementation
│   │
│   └── cortex-prediction-mcp/    # Prediction market MCP server
│       └── src/
│           ├── main.rs           # Entry point (stdio JSON-RPC)
│           ├── config.rs         # Configuration
│           ├── error.rs          # Error types
│           ├── cache.rs          # Query cache (moka)
│           ├── tools.rs          # MCP tool implementations
│           └── db/
│               ├── mod.rs        # ClickHouse client
│               ├── models.rs     # Row types
│               └── queries.rs    # QueryEngine
│
├── migrations/
│   ├── 001_init.sql              # Solana DeFi schema
│   └── 002_prediction_markets.sql # Prediction market schema
│
├── config/
│   └── default.toml              # Default configuration
│
├── docs/
│   └── ARCHITECTURE.md           # Detailed architecture docs
│
├── Dockerfile                    # Multi-binary Docker build
├── docker-compose.yml            # Multi-container deployment
└── API.md                        # REST API documentation
```

## Data Provider: LYS Labs

Cortex uses [LYS Labs](https://lyslabs.ai/) for real-time Solana blockchain data. LYS Labs provides:

- **Ultra-low latency**: Sub-14ms block decoding, sub-30ms enriched data delivery
- **Pre-decoded events**: Human-readable transaction events from DEXs like Raydium, Meteora, Pump.fun
- **WebSocket streaming**: Real-time transaction events as they occur on-chain
- **Rich metadata**: Token swap summaries, volume insights, and liquidity data

### Getting an API Key

1. Visit [https://dev.lyslabs.ai/](https://dev.lyslabs.ai/)
2. Create an account and generate an API key
3. Add the API key to your `.env` file as `CORTEX_LYSLABS_API_KEY`

## License

MIT
