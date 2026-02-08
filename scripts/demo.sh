#!/bin/bash
# Solder Cortex MCP Demo Script
# Demonstrates the unified MCP server capabilities

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           🧠 Solder Cortex MCP Server Demo                 ║${NC}"
echo -e "${BLUE}║        Cross-Domain Intelligence for AI Agents            ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Build the MCP server
echo -e "${YELLOW}📦 Building cortex-unified-mcp...${NC}"
cd "$PROJECT_ROOT"
cargo build -p cortex-unified-mcp --release 2>/dev/null || cargo build -p cortex-unified-mcp

MCP_BIN="$PROJECT_ROOT/target/release/cortex-mcp"
if [ ! -f "$MCP_BIN" ]; then
    MCP_BIN="$PROJECT_ROOT/target/debug/cortex-mcp"
fi

echo -e "${GREEN}✅ Build successful!${NC}"
echo ""

# Function to send JSON-RPC request and get response
send_request() {
    local request="$1"
    echo "$request" | timeout 5 "$MCP_BIN" 2>/dev/null | head -1
}

echo -e "${YELLOW}🔧 Testing MCP Protocol...${NC}"
echo ""

# Test 1: List available tools
echo -e "${BLUE}1. Listing available tools (tools/list)${NC}"
TOOLS_REQUEST='{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
echo "   Request: $TOOLS_REQUEST"
RESPONSE=$(send_request "$TOOLS_REQUEST" || echo '{"error":"timeout or no response"}')
echo "   Response: $RESPONSE"
echo ""

# Test 2: Initialize
echo -e "${BLUE}2. Initialize protocol (initialize)${NC}"
INIT_REQUEST='{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"0.1.0","clientInfo":{"name":"demo","version":"1.0"}}}'
echo "   Request: $INIT_REQUEST"
RESPONSE=$(send_request "$INIT_REQUEST" || echo '{"error":"timeout or no response"}')
echo "   Response: $RESPONSE"
echo ""

# Test 3: Call a DeFi tool (wallet summary - will use default/mock if no real wallet)
echo -e "${BLUE}3. Testing defi_wallet_summary tool${NC}"
WALLET_REQUEST='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"defi_wallet_summary","arguments":{"wallet":"So11111111111111111111111111111111111111112"}}}'
echo "   Request: $WALLET_REQUEST"
RESPONSE=$(send_request "$WALLET_REQUEST" || echo '{"error":"timeout or no response"}')
echo "   Response: $RESPONSE"
echo ""

echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}                    Demo Complete!                          ${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "The Solder Cortex MCP server provides:"
echo -e "  ${BLUE}•${NC} DeFi Analytics (wallet summaries, PnL, positions)"
echo -e "  ${BLUE}•${NC} Prediction Markets (trends, volume profiles, anomalies)"
echo -e "  ${BLUE}•${NC} Cross-Domain Intelligence (conviction scoring)"
echo ""
echo -e "To use with Claude Desktop or other MCP clients, add to config:"
echo -e "  ${YELLOW}$MCP_BIN${NC}"
echo ""
