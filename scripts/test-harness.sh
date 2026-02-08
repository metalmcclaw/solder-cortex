#!/bin/bash
# =============================================================================
# Solder Cortex MCP Test Harness
# =============================================================================
# Automated testing script for the Cortex Unified MCP Server
# Tests all tool categories with JSON-RPC requests and validates responses
#
# Usage: ./scripts/test-harness.sh [--verbose] [--no-build]
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Options
VERBOSE=false
NO_BUILD=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --verbose|-v)
            VERBOSE=true
            ;;
        --no-build)
            NO_BUILD=true
            ;;
        --help|-h)
            echo "Usage: $0 [--verbose] [--no-build]"
            echo ""
            echo "Options:"
            echo "  --verbose, -v    Show detailed output for each test"
            echo "  --no-build       Skip building the MCP server"
            echo "  --help, -h       Show this help message"
            exit 0
            ;;
    esac
done

# =============================================================================
# Helper Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_skip() {
    echo -e "${YELLOW}[SKIP]${NC} $1"
}

log_verbose() {
    if [ "$VERBOSE" = true ]; then
        echo -e "${CYAN}       $1${NC}"
    fi
}

# Send JSON-RPC request to MCP server
# Args: $1 = request JSON
# Returns: response JSON
send_request() {
    local request="$1"
    echo "$request" | timeout 10 "$MCP_BIN" 2>/dev/null | head -1
}

# Validate JSON-RPC response structure
# Args: $1 = response JSON, $2 = test name
# Returns: 0 if valid, 1 if invalid
validate_jsonrpc() {
    local response="$1"
    local test_name="$2"

    # Check if response is valid JSON
    if ! echo "$response" | jq -e . >/dev/null 2>&1; then
        log_fail "$test_name - Invalid JSON response"
        log_verbose "Response: $response"
        return 1
    fi

    # Check JSON-RPC version
    local version
    version=$(echo "$response" | jq -r '.jsonrpc // empty')
    if [ "$version" != "2.0" ]; then
        log_fail "$test_name - Missing or invalid jsonrpc version"
        log_verbose "Got version: $version"
        return 1
    fi

    # Check for id
    if ! echo "$response" | jq -e '.id' >/dev/null 2>&1; then
        log_fail "$test_name - Missing id field"
        return 1
    fi

    # Check for result or error (must have one)
    local has_result has_error
    has_result=$(echo "$response" | jq -e '.result' >/dev/null 2>&1 && echo "yes" || echo "no")
    has_error=$(echo "$response" | jq -e '.error' >/dev/null 2>&1 && echo "yes" || echo "no")

    if [ "$has_result" = "no" ] && [ "$has_error" = "no" ]; then
        log_fail "$test_name - Missing both result and error fields"
        return 1
    fi

    return 0
}

# Run a single test
# Args: $1 = test name, $2 = request JSON, $3 = expected success (true/false), $4 = optional validation function
run_test() {
    local test_name="$1"
    local request="$2"
    local expect_success="${3:-true}"
    local validation_fn="${4:-}"

    log_verbose "Request: $request"

    local response
    response=$(send_request "$request" || echo '{"jsonrpc":"2.0","id":0,"error":{"code":-1,"message":"Server error or timeout"}}')

    log_verbose "Response: $response"

    # Validate JSON-RPC structure
    if ! validate_jsonrpc "$response" "$test_name"; then
        ((TESTS_FAILED++))
        return 1
    fi

    # Check if result/error matches expectation
    local has_error
    has_error=$(echo "$response" | jq -e '.error' >/dev/null 2>&1 && echo "yes" || echo "no")

    if [ "$expect_success" = "true" ] && [ "$has_error" = "yes" ]; then
        log_fail "$test_name - Expected success but got error"
        local error_msg
        error_msg=$(echo "$response" | jq -r '.error.message // "unknown"')
        log_verbose "Error: $error_msg"
        ((TESTS_FAILED++))
        return 1
    fi

    if [ "$expect_success" = "false" ] && [ "$has_error" = "no" ]; then
        log_fail "$test_name - Expected error but got success"
        ((TESTS_FAILED++))
        return 1
    fi

    # Run custom validation if provided
    if [ -n "$validation_fn" ]; then
        if ! $validation_fn "$response"; then
            log_fail "$test_name - Custom validation failed"
            ((TESTS_FAILED++))
            return 1
        fi
    fi

    log_success "$test_name"
    ((TESTS_PASSED++))
    return 0
}

# =============================================================================
# Custom Validators
# =============================================================================

validate_tools_list() {
    local response="$1"
    local tools
    tools=$(echo "$response" | jq '.result.tools')

    # Check tools is array
    if ! echo "$tools" | jq -e 'type == "array"' >/dev/null 2>&1; then
        log_verbose "tools is not an array"
        return 1
    fi

    # Check we have at least 5 tools
    local count
    count=$(echo "$tools" | jq 'length')
    if [ "$count" -lt 5 ]; then
        log_verbose "Expected at least 5 tools, got $count"
        return 1
    fi

    # Check each tool has required fields
    local valid
    valid=$(echo "$tools" | jq 'all(. | has("name") and has("description") and has("inputSchema"))')
    if [ "$valid" != "true" ]; then
        log_verbose "Some tools missing required fields"
        return 1
    fi

    return 0
}

validate_initialize() {
    local response="$1"

    # Check protocol version
    local proto
    proto=$(echo "$response" | jq -r '.result.protocolVersion')
    if [ -z "$proto" ] || [ "$proto" = "null" ]; then
        log_verbose "Missing protocolVersion"
        return 1
    fi

    # Check server info
    local name
    name=$(echo "$response" | jq -r '.result.serverInfo.name')
    if [ "$name" != "cortex-unified-mcp" ]; then
        log_verbose "Unexpected server name: $name"
        return 1
    fi

    return 0
}

validate_health() {
    local response="$1"
    local content
    content=$(echo "$response" | jq -r '.result.content[0].text // empty')

    if [ -z "$content" ]; then
        log_verbose "Missing content text"
        return 1
    fi

    # Parse the health JSON from content
    local health
    if ! health=$(echo "$content" | jq -e '.'); then
        log_verbose "Health content is not valid JSON"
        return 1
    fi

    local status
    status=$(echo "$health" | jq -r '.status')
    if [ "$status" != "healthy" ]; then
        log_verbose "Unexpected status: $status"
        return 1
    fi

    return 0
}

validate_tool_error() {
    local response="$1"

    # Check for isError flag in result
    local is_error
    is_error=$(echo "$response" | jq -r '.result.isError // false')
    if [ "$is_error" != "true" ]; then
        log_verbose "Expected isError=true in result"
        return 1
    fi

    return 0
}

# =============================================================================
# Test Categories
# =============================================================================

test_protocol() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  MCP Protocol Tests${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    # Initialize
    run_test "initialize" \
        '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","clientInfo":{"name":"test-harness","version":"1.0.0"}}}' \
        true \
        validate_initialize

    # Tools list
    run_test "tools/list" \
        '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
        true \
        validate_tools_list

    # Unknown method
    run_test "unknown method (should error)" \
        '{"jsonrpc":"2.0","id":3,"method":"nonexistent/method"}' \
        false
}

test_health() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  Health Check Tests${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    run_test "cortex_health" \
        '{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"cortex_health","arguments":{}}}' \
        true \
        validate_health
}

test_defi_tools() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  DeFi Tool Tests${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    local test_wallet="So11111111111111111111111111111111111111112"

    # Note: These may fail if no backend is running, but should return valid JSON-RPC errors
    run_test "cortex_get_wallet_summary (valid format)" \
        "{\"jsonrpc\":\"2.0\",\"id\":20,\"method\":\"tools/call\",\"params\":{\"name\":\"cortex_get_wallet_summary\",\"arguments\":{\"wallet\":\"$test_wallet\"}}}" \
        true

    run_test "cortex_get_wallet_pnl (valid format)" \
        "{\"jsonrpc\":\"2.0\",\"id\":21,\"method\":\"tools/call\",\"params\":{\"name\":\"cortex_get_wallet_pnl\",\"arguments\":{\"wallet\":\"$test_wallet\",\"window\":\"7d\"}}}" \
        true

    run_test "cortex_get_wallet_positions (valid format)" \
        "{\"jsonrpc\":\"2.0\",\"id\":22,\"method\":\"tools/call\",\"params\":{\"name\":\"cortex_get_wallet_positions\",\"arguments\":{\"wallet\":\"$test_wallet\"}}}" \
        true

    run_test "cortex_list_subscriptions" \
        '{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{"name":"cortex_list_subscriptions","arguments":{}}}' \
        true
}

test_error_handling() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  Error Handling Tests${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    # Missing required parameter
    run_test "missing wallet parameter" \
        '{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"cortex_get_wallet_summary","arguments":{}}}' \
        true \
        validate_tool_error

    # Invalid wallet format
    run_test "invalid wallet format (too short)" \
        '{"jsonrpc":"2.0","id":31,"method":"tools/call","params":{"name":"cortex_get_wallet_summary","arguments":{"wallet":"invalid"}}}' \
        true \
        validate_tool_error

    # Unknown tool
    run_test "unknown tool name" \
        '{"jsonrpc":"2.0","id":32,"method":"tools/call","params":{"name":"nonexistent_tool","arguments":{}}}' \
        true \
        validate_tool_error

    # Empty wallet
    run_test "empty wallet parameter" \
        '{"jsonrpc":"2.0","id":33,"method":"tools/call","params":{"name":"cortex_get_wallet_summary","arguments":{"wallet":""}}}' \
        true \
        validate_tool_error
}

test_cross_domain() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  Cross-Domain Intelligence Tests${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    # Conviction with Solana wallet
    run_test "cortex_get_wallet_conviction (Solana)" \
        '{"jsonrpc":"2.0","id":40,"method":"tools/call","params":{"name":"cortex_get_wallet_conviction","arguments":{"wallet":"So11111111111111111111111111111111111111112"}}}' \
        true

    # Conviction with EVM wallet
    run_test "cortex_get_wallet_conviction (EVM)" \
        '{"jsonrpc":"2.0","id":41,"method":"tools/call","params":{"name":"cortex_get_wallet_conviction","arguments":{"wallet":"0x1234567890123456789012345678901234567890"}}}' \
        true

    # Conviction with linked EVM
    run_test "cortex_get_wallet_conviction (linked)" \
        '{"jsonrpc":"2.0","id":42,"method":"tools/call","params":{"name":"cortex_get_wallet_conviction","arguments":{"wallet":"So11111111111111111111111111111111111111112","evm_address":"0x1234567890123456789012345678901234567890"}}}' \
        true

    # Informed trader detection
    run_test "cortex_detect_informed_traders" \
        '{"jsonrpc":"2.0","id":43,"method":"tools/call","params":{"name":"cortex_detect_informed_traders","arguments":{"market_slug":"test-market","platform":"polymarket","min_conviction":0.5}}}' \
        true
}

test_prediction_tools() {
    echo ""
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}  Prediction Market Tests (may skip if no Clickhouse)${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    # These tools require Clickhouse - they should return valid JSON-RPC even on error
    run_test "cortex_get_market_trend" \
        '{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"cortex_get_market_trend","arguments":{"slug":"test-market","interval":"1h"}}}' \
        true

    run_test "cortex_get_volume_profile" \
        '{"jsonrpc":"2.0","id":51,"method":"tools/call","params":{"name":"cortex_get_volume_profile","arguments":{"slug":"test-market"}}}' \
        true

    run_test "cortex_search_market_memory" \
        '{"jsonrpc":"2.0","id":52,"method":"tools/call","params":{"name":"cortex_search_market_memory","arguments":{"query":"bitcoin","limit":10}}}' \
        true

    run_test "cortex_detect_anomalies" \
        '{"jsonrpc":"2.0","id":53,"method":"tools/call","params":{"name":"cortex_detect_anomalies","arguments":{"slug":"test-market","threshold":3.0}}}' \
        true
}

# =============================================================================
# Main
# =============================================================================

main() {
    echo ""
    echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║         🧪 Solder Cortex MCP Test Harness                  ║${NC}"
    echo -e "${BLUE}║            Automated Integration Testing                   ║${NC}"
    echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Build MCP server
    if [ "$NO_BUILD" = false ]; then
        log_info "Building cortex-unified-mcp..."
        cd "$PROJECT_ROOT"
        if cargo build -p cortex-unified-mcp --release 2>/dev/null; then
            log_success "Build successful (release)"
        elif cargo build -p cortex-unified-mcp 2>/dev/null; then
            log_success "Build successful (debug)"
        else
            log_fail "Build failed"
            exit 1
        fi
    fi

    # Find binary
    MCP_BIN="$PROJECT_ROOT/target/release/cortex-mcp"
    if [ ! -f "$MCP_BIN" ]; then
        MCP_BIN="$PROJECT_ROOT/target/debug/cortex-mcp"
    fi

    if [ ! -f "$MCP_BIN" ]; then
        log_fail "MCP binary not found. Run with --no-build only if already built."
        exit 1
    fi

    log_info "Using binary: $MCP_BIN"
    echo ""

    # Check for jq
    if ! command -v jq &> /dev/null; then
        log_fail "jq is required but not installed. Install with: apt install jq"
        exit 1
    fi

    # Run test categories
    test_protocol
    test_health
    test_defi_tools
    test_error_handling
    test_cross_domain
    test_prediction_tools

    # Summary
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  Test Summary${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "  ${GREEN}Passed:${NC}  $TESTS_PASSED"
    echo -e "  ${RED}Failed:${NC}  $TESTS_FAILED"
    echo -e "  ${YELLOW}Skipped:${NC} $TESTS_SKIPPED"
    echo ""

    TOTAL=$((TESTS_PASSED + TESTS_FAILED))
    if [ $TESTS_FAILED -eq 0 ]; then
        echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${GREEN}║                  ✅ ALL TESTS PASSED!                       ║${NC}"
        echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
        exit 0
    else
        echo -e "${RED}╔════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${RED}║                  ❌ SOME TESTS FAILED                       ║${NC}"
        echo -e "${RED}╚════════════════════════════════════════════════════════════╝${NC}"
        exit 1
    fi
}

# Run main
main
