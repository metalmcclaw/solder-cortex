# Cortex Stack Architecture

## Overview

Cortex is a Rust-based memory layer for AI agents, providing real-time and historical financial data through MCP (Model Context Protocol) servers. The stack consists of multiple crates organized in a monorepo format.

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
│  • list_subscriptions     │        │                                   │
└───────────┬───────────────┘        └─────────────┬─────────────────────┘
            │ HTTP REST                            │ Direct Clickhouse
            ▼                                      ▼
┌───────────────────────────┐        ┌───────────────────────────────────┐
│      cortex-server        │        │         Query Cache               │
│    (REST API Server)      │        │         (moka async)              │
│                           │        │                                   │
│  Endpoints:               │        │  • Market trend cache             │
│  • GET /health            │        │  • Volume profile cache           │
│  • GET /user/{w}/summary  │        │  • Search results cache           │
│  • GET /user/{w}/pnl      │        │  • Anomaly cache                  │
│  • GET /user/{w}/positions│        │  • TTL: 60s (configurable)        │
│  • POST /index            │        └─────────────┬─────────────────────┘
│  • DELETE /index/{wallet} │                      │
└───────────┬───────────────┘                      │
            │                                      │
            ▼                                      │
┌───────────────────────────┐                      │
│       Indexer             │                      │
│                           │                      │
│  Phase 1: Historical      │                      │
│  └─ Helius API            │                      │
│                           │                      │
│  Phase 2: Real-time       │                      │
│  └─ LYS Labs WebSocket    │                      │
└───────────┬───────────────┘                      │
            │                                      │
            │              ┌───────────────────────┘
            │              │
            ▼              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                            CLICKHOUSE                                    │
│                                                                          │
│  Solana DeFi Tables:              Prediction Market Tables:              │
│  ┌─────────────────────┐          ┌──────────────────────────┐          │
│  │ transactions        │          │ markets                  │          │
│  │ positions           │          │ market_prices            │          │
│  │ wallet_summaries    │          │ market_trades            │          │
│  │ token_prices        │          │ market_volume            │          │
│  └─────────────────────┘          │ market_orderbook         │          │
│                                   │ market_stats             │          │
│                                   │ mv_market_volume_1h (MV) │          │
│                                   └──────────────────────────┘          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
cortex/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── cortex-server/            # Main API server + indexer
│   │   └── src/
│   │       ├── main.rs           # Entry point
│   │       ├── config.rs         # Configuration
│   │       ├── error.rs          # Error types
│   │       ├── types.rs          # Domain types (Protocol, TransactionType)
│   │       ├── api/              # HTTP handlers
│   │       ├── db/               # Clickhouse queries
│   │       ├── indexer/          # LYS Labs + Helius clients
│   │       │   └── protocols/    # Protocol parsers (Jupiter, Raydium, Kamino)
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
│               ├── mod.rs        # Clickhouse client
│               ├── models.rs     # Row types
│               └── queries.rs    # QueryEngine with parameterized SQL
│
├── migrations/
│   ├── 001_init.sql              # Solana DeFi schema
│   └── 002_prediction_markets.sql # Prediction market schema
│
├── config/
│   └── default.toml              # Default configuration
│
└── docker-compose.yml            # Multi-container deployment
```

## Data Flow

### Solana DeFi Flow (cortex-server + cortex-mcp)

```
1. User calls MCP tool (e.g., cortex_start_indexing)
2. cortex-mcp forwards to cortex-server REST API
3. cortex-server starts hybrid indexing:
   a. Helius API fetches historical transactions
   b. LYS Labs WebSocket streams real-time transactions
4. Parser identifies protocol (Jupiter, Raydium, Kamino, etc.)
5. Transactions stored in Clickhouse
6. Subsequent queries (summary, pnl, positions) read from DB
```

### Prediction Market Flow (cortex-prediction-mcp)

```
1. User calls MCP tool (e.g., get_market_trend)
2. Check moka cache for recent results
3. If cache miss, execute parameterized Clickhouse query
4. Format response as agent-readable JSON
5. Cache result for future requests
6. Return to AI agent
```

## Supported Protocols

### Solana DeFi
| Protocol | Type | Operations |
|----------|------|------------|
| Jupiter | DEX Aggregator | Swaps |
| Raydium | AMM/CLMM | Swaps, LP |
| Kamino | Lending | Supply, Borrow, Withdraw, Repay |
| Meteora | DLMM DEX | Swaps |
| Orca | Whirlpool | Swaps |
| Pump.fun | Token Launchpad | Swaps |

### Prediction Markets
| Platform | Data Types |
|----------|------------|
| Polymarket | Prices, Trades, Order Book |
| Kalshi | Prices, Trades, Order Book |

## Configuration

### Environment Variables

See `.env.example` for full list. Key prefixes:
- `CORTEX__` - cortex-server configuration
- `CORTEX_PREDICTION__` - cortex-prediction-mcp configuration

### Hierarchical Config Loading

1. Hardcoded defaults in code
2. `config/default.toml` (if exists)
3. `config/local.toml` (if exists, for local overrides)
4. Environment variables (highest priority)

## Deployment

### Docker Compose Services

| Service | Port | Purpose |
|---------|------|---------|
| cortex | 3000 | Main API server |
| cortex-prediction-mcp | stdio | Prediction market MCP |
| clickhouse | 8123, 9000 | Database |
| swagger-ui | 8080 | API documentation |
| clickhouse-ui | 8081 | Database UI |

### Running Locally

```bash
# Start infrastructure
docker-compose up -d clickhouse clickhouse-ui swagger-ui

# Run cortex-server
cargo run -p cortex-server

# Run prediction MCP (in another terminal)
cargo run -p cortex-prediction-mcp
```

## Key Design Decisions

1. **Pure JSON-RPC for MCP**: Avoids complex SDK dependencies, maximizes compatibility
2. **Direct Clickhouse Access**: cortex-prediction-mcp queries DB directly for performance
3. **moka Cache**: In-memory async cache reduces DB load for frequent queries
4. **Parameterized Queries**: Prevents SQL injection, enables query plan caching
5. **Monorepo Structure**: Shared workspace config, unified dependency management
6. **Hybrid Indexing**: Historical backfill + real-time streaming for complete data
