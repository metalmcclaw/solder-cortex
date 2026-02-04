# Solder Cortex API Documentation

Solder Cortex provides a pre-indexed memory layer for AI DeFi agents on Solana. These APIs are optimized for agent consumption - clean JSON, fast responses, and pre-computed metrics.

## Base URL

```
http://localhost:3000
```

## Data Provider

Cortex uses **LYS Labs** for real-time Solana blockchain data via WebSocket streaming. This provides:
- Sub-second latency on transaction events
- Pre-decoded DeFi transactions (swaps, deposits, borrows)
- Support for major DEXs: Jupiter, Raydium, Meteora, Orca, Pump.fun

## Endpoints

### Health Check

```
GET /health
```

Returns service health status.

**Response:**
```json
{
  "status": "ok",
  "version": "0.1.0",
  "database": "connected"
}
```

---

### Get User Summary

```
GET /api/v1/user/{wallet}/summary
```

Returns high-level wallet state including PnL, risk metrics, and protocol exposure.

**Path Parameters:**
- `wallet` (string, required): Solana wallet address (base58)

**Response:**
```json
{
  "wallet": "95n9a8yd6aZzKGMtbWSjqbijZ1u99z1GQF79HkbCvtwN",
  "total_value_usd": 15420.50,
  "pnl": {
    "realized_24h": 120.30,
    "realized_7d": 540.00,
    "realized_30d": 1250.00,
    "unrealized": 890.25
  },
  "risk": {
    "score": 45,
    "largest_position_pct": 0.35,
    "protocol_count": 3
  },
  "last_activity": "2026-01-14T10:30:00Z",
  "protocols": ["jupiter", "raydium", "kamino"]
}
```

**Risk Score Interpretation:**
- 0-25: Low risk (diversified, multiple protocols)
- 26-50: Moderate risk (some concentration)
- 51-75: Elevated risk (high concentration)
- 76-100: High risk (single position/protocol dominance)

---

### Get User PnL

```
GET /api/v1/user/{wallet}/pnl?window={timeframe}
```

Returns detailed PnL breakdown by protocol within a time window.

**Path Parameters:**
- `wallet` (string, required): Solana wallet address

**Query Parameters:**
- `window` (string, optional): Time window - `24h`, `7d`, `30d`, or `all`. Default: `7d`

**Response:**
```json
{
  "wallet": "95n9a8yd6aZzKGMtbWSjqbijZ1u99z1GQF79HkbCvtwN",
  "window": "7d",
  "total_realized": 540.00,
  "total_unrealized": 890.25,
  "by_protocol": [
    {
      "protocol": "jupiter",
      "realized": 320.00,
      "unrealized": 0,
      "trade_count": 12
    },
    {
      "protocol": "kamino",
      "realized": 220.00,
      "unrealized": 890.25,
      "trade_count": 3
    }
  ]
}
```

---

### Get User Positions

```
GET /api/v1/user/{wallet}/positions
```

Returns all current open positions across supported protocols.

**Path Parameters:**
- `wallet` (string, required): Solana wallet address

**Response:**
```json
{
  "wallet": "95n9a8yd6aZzKGMtbWSjqbijZ1u99z1GQF79HkbCvtwN",
  "positions": [
    {
      "protocol": "kamino",
      "type": "lending_supply",
      "token": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
      "amount": 5000.0,
      "usd_value": 5000.0,
      "apy": 0.082,
      "unrealized_pnl": 0
    },
    {
      "protocol": "raydium",
      "type": "lp",
      "token": "SOL-USDC",
      "pool": "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2",
      "amount": 2.5,
      "usd_value": 2500.0,
      "unrealized_pnl": 150.25
    }
  ],
  "total_value_usd": 7500.0
}
```

**Position Types:**
- `lending_supply`: Tokens supplied to lending protocol
- `lending_borrow`: Tokens borrowed from lending protocol
- `lp`: Liquidity pool position

---

### Index Wallet

```
POST /api/v1/index
```

Triggers real-time indexing for a wallet. This connects to the LYS Labs WebSocket stream and collects transactions for the specified wallet.

**Note:** Since LYS Labs provides real-time streaming data, this endpoint streams transactions as they occur. The indexing process will collect transactions for a configurable timeout period (default: 60 seconds) or until the transaction limit is reached.

**Request Body:**
```json
{
  "wallet": "95n9a8yd6aZzKGMtbWSjqbijZ1u99z1GQF79HkbCvtwN"
}
```

**Response:**
```json
{
  "wallet": "95n9a8yd6aZzKGMtbWSjqbijZ1u99z1GQF79HkbCvtwN",
  "status": "indexing",
  "message": "Wallet indexing started. Streaming real-time transactions."
}
```

---

## Supported Protocols

| Protocol | Type | Program ID | Supported Operations |
|----------|------|------------|---------------------|
| Jupiter | DEX Aggregator | `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4` | Swaps |
| Raydium | DEX (AMM/CLMM) | `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` | Swaps, LP |
| Kamino | Lending | `KLend2g3cP87ber41L3rfCMYbkK3YqPjSSahS1E3HVK` | Supply, Borrow, Withdraw, Repay |
| Meteora | DEX (DLMM) | `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo` | Swaps |
| Orca | DEX (Whirlpool) | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | Swaps |
| Pump.fun | Token Launchpad | `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P` | Swaps |

---

## Error Responses

All errors return JSON with `error` and `code` fields:

```json
{
  "error": "Invalid wallet address: xyz",
  "code": "INVALID_WALLET"
}
```

**Error Codes:**
- `WALLET_NOT_FOUND` (404): Wallet has no indexed data
- `INVALID_WALLET` (400): Invalid Solana address format
- `INVALID_PARAM` (400): Invalid query parameter
- `DATABASE_ERROR` (500): ClickHouse connection issue
- `EXTERNAL_API_ERROR` (502): LYS Labs WebSocket connection issue
- `INTERNAL_ERROR` (500): Unexpected server error

---

## Configuration

Environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `CORTEX_SERVER_HOST` | Server bind address | `0.0.0.0` |
| `CORTEX_SERVER_PORT` | Server port | `3000` |
| `CORTEX_DATABASE_URL` | ClickHouse URL | `http://localhost:8123` |
| `CORTEX_DATABASE_DATABASE` | ClickHouse database | `cortex` |
| `CORTEX_LYSLABS_API_KEY` | LYS Labs API key | (required) |
| `CORTEX_LYSLABS_WS_URL` | LYS Labs WebSocket URL | `wss://solana-mainnet-api-vip.lyslabs.ai/v1/` |
| `RUST_LOG` | Log level | `cortex=info` |

---

## Data Flow Architecture

```
+-------------------+     WebSocket      +-------------------+
|                   |    Connection      |                   |
|    LYS Labs       | <----------------> |    Cortex         |
|    (Solana Data)  |    Real-time       |    Indexer        |
|                   |    Streaming       |                   |
+-------------------+                    +-------------------+
                                                  |
                                                  | Parse & Filter
                                                  v
                                         +-------------------+
                                         |                   |
                                         |   Protocol        |
                                         |   Parsers         |
                                         |                   |
                                         +-------------------+
                                                  |
                                                  | Store
                                                  v
                                         +-------------------+
                                         |                   |
                                         |   ClickHouse      |
                                         |   (Analytics DB)  |
                                         |                   |
                                         +-------------------+
                                                  |
                                                  | Query
                                                  v
                                         +-------------------+
                                         |                   |
                                         |   REST API        |
                                         |   (Axum)          |
                                         |                   |
                                         +-------------------+
                                                  |
                                                  | JSON Response
                                                  v
                                         +-------------------+
                                         |                   |
                                         |   AI Agent        |
                                         |                   |
                                         +-------------------+
```

### How Indexing Works

1. **WebSocket Connection**: When you call `POST /api/v1/index`, Cortex establishes a WebSocket connection to LYS Labs.

2. **Subscribe to Stream**: The indexer subscribes to the real-time transaction stream with `{ "action": "subscribe" }`.

3. **Filter by Wallet**: Incoming transactions are filtered to only include those involving the specified wallet address.

4. **Parse Transactions**: Each relevant transaction is parsed to extract:
   - Protocol (Jupiter, Raydium, Kamino, etc.)
   - Transaction type (Swap, Deposit, Withdraw, etc.)
   - Token amounts and addresses
   - Timestamps and signatures

5. **Store & Compute**: Transactions are stored in ClickHouse, and metrics (PnL, risk) are computed.

6. **Serve via API**: Pre-computed data is served through the REST API endpoints.

---

## Quick Start

```bash
# 1. Set up environment
cp .env.example .env
# Edit .env and add your LYS Labs API key

# 2. Start ClickHouse and Cortex
docker-compose up -d

# 3. Index a wallet (streams real-time transactions)
curl -X POST http://localhost:3000/api/v1/index \
  -H "Content-Type: application/json" \
  -d '{"wallet": "YOUR_WALLET_ADDRESS"}'

# 4. Query wallet summary
curl http://localhost:3000/api/v1/user/YOUR_WALLET_ADDRESS/summary
```

---

## LYS Labs Integration

Cortex uses LYS Labs' WebSocket API for real-time Solana data. Key characteristics:

- **Connection URL**: `wss://solana-mainnet-api-vip.lyslabs.ai/v1/?apiKey=YOUR_API_KEY`
- **Authentication**: API key passed as URL query parameter
- **Message Format**: JSON with `type` field indicating message type (`transaction`, `transactions`)
- **Subscription**: Send `{ "action": "subscribe" }` to start receiving events

### Transaction Event Structure

LYS Labs provides decoded transaction events with the following structure:

```json
{
  "type": "transaction",
  "data": {
    "txSignature": "5abc...",
    "slot": 123456789,
    "blockTime": 1705234567,
    "decoderType": "RAYDIUM",
    "eventType": "SWAP",
    "mint": "TokenMintAddress...",
    "source": "SourceWallet...",
    "destination": "DestWallet...",
    "tokenIn": {
      "mint": "...",
      "amount": "1000000",
      "uiAmount": 1.0,
      "decimals": 6
    },
    "tokenOut": {
      "mint": "...",
      "amount": "500000000",
      "uiAmount": 0.5,
      "decimals": 9
    }
  }
}
```

For more information about LYS Labs, visit [https://docs.lyslabs.ai/](https://docs.lyslabs.ai/).
