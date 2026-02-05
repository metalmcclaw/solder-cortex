# Cortex Unified MCP Server

The unified Model Context Protocol (MCP) server for Solder Cortex, providing cross-domain intelligence for AI agents by combining DeFi analytics and prediction market data.

## Overview

This server unifies the previously separate `cortex-mcp` and `cortex-prediction-mcp` servers into a single entry point. AI agents now only need one MCP connection to access all Cortex capabilities.

## Features

### DeFi Analytics
- **Wallet Summary** - Portfolio value, risk metrics, protocol exposure
- **Wallet PnL** - Profit/loss breakdown by protocol
- **Wallet Positions** - All open DeFi positions
- **Indexing** - Start/stop real-time wallet monitoring

### Prediction Markets
- **Market Trends** - OHLCV data with volume
- **Volume Profiles** - Liquidity and trading metrics
- **Market Search** - Historical market lookup
- **Anomaly Detection** - Price spike detection

### Cross-Domain Intelligence
- **Wallet Conviction** - Correlate DeFi positions with prediction bets
- **Informed Traders** - Detect traders with aligned on-chain activity

## Installation

```bash
cd crates/cortex-unified-mcp
cargo build --release
```

## Configuration

Environment variables:
```bash
# DeFi API
CORTEX_API_URL=http://localhost:3000

# Prediction Markets (optional)
CORTEX_PREDICTION_ENABLED=true
CLICKHOUSE_URL=http://localhost:8123
CLICKHOUSE_DATABASE=cortex
```

## Usage

### As MCP Server

```bash
# Run as MCP server (stdio)
./target/release/cortex-mcp
```

### Claude Desktop Integration

Add to `claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "cortex": {
      "command": "/path/to/cortex-mcp",
      "env": {
        "CORTEX_API_URL": "http://localhost:3000"
      }
    }
  }
}
```

## Tools

### DeFi Tools
| Tool | Description |
|------|-------------|
| `cortex_health` | Health check |
| `cortex_get_wallet_summary` | Wallet overview |
| `cortex_get_wallet_pnl` | PnL breakdown |
| `cortex_get_wallet_positions` | Open positions |
| `cortex_start_indexing` | Start monitoring |
| `cortex_stop_indexing` | Stop monitoring |
| `cortex_list_subscriptions` | List monitored wallets |

### Cross-Domain Tools
| Tool | Description |
|------|-------------|
| `cortex_get_wallet_conviction` | Conviction analysis |
| `cortex_detect_informed_traders` | Find informed traders |

### Prediction Tools (when Clickhouse available)
| Tool | Description |
|------|-------------|
| `cortex_get_market_trend` | Price trends |
| `cortex_get_volume_profile` | Volume analysis |
| `cortex_search_market_memory` | Historical search |
| `cortex_detect_anomalies` | Anomaly detection |

## Architecture

```
┌─────────────────────────────────────────┐
│         AI Agent (Claude, etc.)         │
└─────────────────┬───────────────────────┘
                  │ MCP Protocol (stdio)
                  ▼
┌─────────────────────────────────────────┐
│       Cortex Unified MCP Server         │
│  ┌────────────┐  ┌─────────────────┐    │
│  │ DeFi Client│  │Prediction Engine│    │
│  └──────┬─────┘  └────────┬────────┘    │
└─────────┼─────────────────┼─────────────┘
          │                 │
          ▼                 ▼
    ┌──────────┐     ┌────────────┐
    │ Cortex   │     │ Clickhouse │
    │ DeFi API │     │ (optional) │
    └──────────┘     └────────────┘
```

## License

MIT
