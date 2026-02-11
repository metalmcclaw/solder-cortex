# Prediction Markets API Implementation

## Required Endpoints

### 1. Active Markets (Polymarket + Kalshi)
```
GET /api/v1/markets/trending
```

Response:
```json
{
  "markets": [
    {
      "platform": "polymarket",
      "slug": "trump-wins-2024",
      "title": "Trump Wins 2024",
      "category": "politics",
      "current_price": 0.60,
      "change_24h": 0.082,
      "volume_24h": 2400000,
      "trades_24h": 3247,
      "end_date": "2024-11-05T00:00:00Z"
    }
  ]
}
```

### 2. Market Details
```
GET /api/v1/markets/{platform}/{slug}
```

### 3. Wallet Prediction Positions
```
GET /api/v1/user/{wallet}/prediction-positions
```

For EVM wallets (Polymarket):
```
GET /api/v1/user/{evm_address}/polymarket-positions
```

### 4. Cross-Domain Conviction
```
GET /api/v1/user/{wallet}/conviction?evm_address={evm}
```

## Implementation Plan

1. ✅ Prediction clients exist (polymarket.rs, kalshi.rs)
2. 🔄 Add API routes to Rust server 
3. 🔄 Update dashboard to call real endpoints
4. ❌ Remove all mock/demo data from dashboard

## Dashboard Updates

Replace mock market data with real API calls:
- Load trending markets from `/api/v1/markets/trending`
- Show real prices, volumes, changes
- Support both Polymarket and Kalshi data
- Error handling when APIs are down (show "N/A" not demo data)