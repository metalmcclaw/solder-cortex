# Solder Cortex Dashboard 🤘

Interactive demo dashboard showcasing the 13 MCP tools for cross-domain intelligence.

## Live Demo

Visit: https://metalmcclaw.github.io/solder-cortex/dashboard/

## Features

### 📊 Overview
- Health status of all Cortex backends
- Quick stats (tracked value, PnL, protocols)
- Active wallet subscriptions

### 🔧 DeFi Tools (7 tools)
- `cortex_health` - Service health check
- `cortex_get_wallet_summary` - Comprehensive wallet overview
- `cortex_get_wallet_pnl` - PnL breakdown by protocol
- `cortex_get_wallet_positions` - Current open positions
- `cortex_start_indexing` - Start wallet indexing
- `cortex_stop_indexing` - Stop wallet indexing
- `cortex_list_subscriptions` - List indexed wallets

### 💼 Portfolio View
- Aggregated position display
- Protocol allocation breakdown
- Visual charts and metrics

### 🎯 Cross-Domain Conviction (2 tools)
- `cortex_get_wallet_conviction` - Cross-domain conviction scoring
- `cortex_detect_informed_traders` - Find wallets with edge

### 🔮 Prediction Markets (4 tools)
- `cortex_get_market_trend` - OHLCV data with intervals
- `cortex_get_volume_profile` - Volume and liquidity
- `cortex_search_market_memory` - Search historical markets
- `cortex_detect_anomalies` - Find price spikes

### 🏛️ DAO Governance
- Roadmap for governance integration
- MetaDAO futarchy preview

## Local Development

Just open `index.html` in a browser - it's a single self-contained file.

```bash
cd dashboard
python3 -m http.server 8080
# Visit http://localhost:8080
```

## Deployment

This is designed to deploy alongside the main landing page:

```
website/
├── index.html      # Landing page
├── pitch/          # Pitch deck
└── dashboard/      # This dashboard
```

For GitHub Pages, the dashboard will be at `/solder-cortex/dashboard/`

## Demo Walkthrough (2-3 min)

1. **Overview** (30s)
   - Show health status, quick stats
   - Point out 13 MCP tools
   
2. **DeFi Tools** (45s)
   - Show wallet summary response
   - Explain PnL and positions

3. **Conviction** (60s) ⭐ Key differentiator
   - Show cross-domain conviction scoring
   - Explain how DeFi positions + prediction bets = conviction
   - Show informed trader detection

4. **Prediction Markets** (30s)
   - Quick tour of market tools
   - Show anomaly detection

5. **Future** (15s)
   - DAO governance roadmap

## Branding

Matches the main landing page:
- Dark theme: `#163300` primary, `#0f2200` secondary
- Accent: `#C9FF99` (lime green)
- Font: Asap
- Logo: 🤘 Solder Cortex
