import sys
import json
import subprocess
import threading
import time

def read_output(process):
    while True:
        line = process.stdout.readline()
        if not line:
            break
        try:
            msg = json.loads(line)
            # print(f"DEBUG: Received: {msg}")
        except json.JSONDecodeError:
            pass
            # print(f"DEBUG: Raw: {line}")

def send_request(process, method, params=None, id=1):
    req = {
        "jsonrpc": "2.0",
        "method": method,
        "params": params or {},
        "id": id
    }
    process.stdin.write(json.dumps(req) + "\n")
    process.stdin.flush()

def main():
    print("🤖 Starting Solder Cortex Chatbot...")
    
    # Path to the compiled MCP server binary
    server_path = "./target/release/cortex-mcp"
    
    try:
        process = subprocess.Popen(
            [server_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1
        )
    except FileNotFoundError:
        print(f"❌ Error: Could not find MCP server at {server_path}")
        print("Make sure you've run 'cargo build -p cortex-unified-mcp --release'")
        return

    # Start a thread to read output (for debugging/logging)
    # threading.Thread(target=read_output, args=(process,), daemon=True).start()

    # Initialize
    print("🔌 Connecting to MCP Server...")
    send_request(process, "initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "chatbot", "version": "0.1.0"}
    }, id=0)

    # Read initialization response
    while True:
        line = process.stdout.readline()
        if not line: break
        try:
            msg = json.loads(line)
            if msg.get("id") == 0:
                print("✅ Connected! Server capabilities:", msg.get("result", {}).get("capabilities"))
                break
        except: pass
    
    # Notify initialized
    send_request(process, "notifications/initialized")

    print("\n💬 Chatbot ready! Type 'exit' to quit.")
    print("Try asking about: 'price of SOL', 'my balance', 'swap SOL to USDC'")

    msg_id = 1
    while True:
        try:
            user_input = input("\nYou: ").strip()
        except EOFError:
            break
            
        if user_input.lower() in ["exit", "quit"]:
            break
        
        if not user_input:
            continue

        # Simple intent matching for demo purposes
        tool_name = None
        tool_args = {}

        if "health" in user_input.lower():
            tool_name = "cortex_health"
            tool_args = {}

        elif "balance" in user_input.lower() or "summary" in user_input.lower():
            # Use a dummy wallet for demo if none provided
            wallet = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1" 
            if "0x" in user_input: wallet = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e"
            tool_name = "cortex_get_wallet_summary"
            tool_args = {"wallet": wallet}
            
        elif "positions" in user_input.lower():
            wallet = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1"
            tool_name = "cortex_get_wallet_positions"
            tool_args = {"wallet": wallet}
            
        elif "conviction" in user_input.lower():
            wallet = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1"
            tool_name = "cortex_get_wallet_conviction"
            tool_args = {"wallet": wallet}

        elif "trend" in user_input.lower() or "market" in user_input.lower():
            slug = "will-btc-hit-100k-in-2024"
            if "sol" in user_input.lower(): slug = "will-solana-reach-500-in-2025"
            tool_name = "cortex_get_market_trend"
            tool_args = {"slug": slug, "interval": "24h"}
            
        else:
            print("🤖 Agent: I can check health, wallet balances, positions, conviction scores, and market trends.")
            print("   (Try: 'health', 'wallet balance', 'market trend for SOL')")
            continue

        print(f"🤖 Agent: invoking tool `{tool_name}` with {tool_args}...")
        
        send_request(process, "tools/call", {
            "name": tool_name,
            "arguments": tool_args
        }, id=msg_id)
        
        # Read response
        while True:
            line = process.stdout.readline()
            if not line: break
            try:
                msg = json.loads(line)
                if msg.get("id") == msg_id:
                    if "error" in msg:
                        print(f"❌ Error: {msg['error']['message']}")
                    else:
                        result = msg.get("result", {})
                        content = result.get("content", [])
                        for item in content:
                            if item.get("type") == "text":
                                print(f"✅ Tool Output: {item.get('text')}")
                    break
            except json.JSONDecodeError:
                pass
        
        msg_id += 1

    process.terminate()
    print("👋 Bye!")

if __name__ == "__main__":
    main()
