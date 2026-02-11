#!/bin/bash
# Deploy Market-Wide Indexer for Solder Cortex
# Run this to switch from wallet-centric to market-wide data aggregation

set -e

echo "🤘 SOLDER CORTEX - MARKET INDEXER DEPLOYMENT"
echo "=============================================="

# 1. Database Migration
echo "[DB] Running market aggregation migration..."
clickhouse-client --multiquery < migrations/003_market_aggregation.sql || {
    echo "❌ Migration failed - check ClickHouse connection"
    exit 1
}
echo "✅ Database schema updated"

# 2. Build new version
echo "[BUILD] Compiling market-focused version..."
cargo build --release || {
    echo "❌ Build failed - check Rust compilation errors"
    exit 1
}
echo "✅ Binary compiled successfully"

# 3. Stop old indexer
echo "[DEPLOY] Stopping old wallet-focused indexer..."
pkill -f "./cortex" || echo "No existing process found"

# 4. Start new market indexer
echo "[DEPLOY] Starting market-wide indexer..."
cd target/release
nohup ./cortex > cortex.log 2>&1 &
NEW_PID=$!
echo "✅ New indexer started (PID: $NEW_PID)"

# 5. Health check
echo "[HEALTH] Checking new deployment..."
sleep 5

if curl -s http://localhost:3001/health | grep -q "ok"; then
    echo "✅ Health check passed"
else
    echo "❌ Health check failed - check logs"
    tail -20 cortex.log
    exit 1
fi

# 6. Test new v2 API
echo "[API] Testing market overview endpoint..."
if curl -s http://localhost:3001/api/v2/market/overview | grep -q "defi"; then
    echo "✅ Market API working"
else
    echo "⚠️  Market API not responding - check logs"
fi

echo ""
echo "🎯 DEPLOYMENT COMPLETE!"
echo "• Legacy v1 API: /api/v1/* (deprecated)"  
echo "• New Market API: /api/v2/market/*"
echo "• Dashboard: Update to use v2 endpoints"
echo "• Logs: target/release/cortex.log"
echo ""
echo "Next: Update dashboard to use /api/v2/market/overview"