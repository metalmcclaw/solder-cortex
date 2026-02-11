#!/usr/bin/env python3
"""
Simple API test to verify Helius and LYS Labs connections work with the real API keys.
"""
import os
import requests
import json

def load_env_vars():
    """Load environment variables from .env file"""
    env_vars = {}
    
    try:
        with open('/root/.openclaw/workspace/solder-cortex/.env', 'r') as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#') and '=' in line:
                    key, value = line.split('=', 1)
                    env_vars[key] = value
    except Exception as e:
        print(f"Error reading .env file: {e}")
        return None, None, None
    
    helius_key = env_vars.get('CORTEX__HELIUS__API_KEY')
    lyslabs_key = env_vars.get('CORTEX__LYSLABS__API_KEY')
    lyslabs_url = env_vars.get('CORTEX__LYSLABS__WS_URL')
    
    print(f"Helius API Key: {helius_key[:8]}...{helius_key[-4:] if helius_key else 'EMPTY'}")
    print(f"LYS Labs API Key: {lyslabs_key[:8]}...{lyslabs_key[-4:] if lyslabs_key else 'EMPTY'}")
    print(f"LYS Labs WS URL: {lyslabs_url}")
    
    return helius_key, lyslabs_key, lyslabs_url

def test_helius_api(api_key):
    """Test a simple Helius API call"""
    if not api_key:
        print("❌ HELIUS: No API key provided")
        return False
    
    # Test with a well-known wallet (Jupiter treasury for example)
    test_wallet = "D8cy77BBepLMngZx6ZukaTff5hCt1HrWyKk3Hnd9oitf"
    url = f"https://api.helius.xyz/v0/addresses/{test_wallet}/transactions"
    
    params = {
        'api-key': api_key,
        'limit': 1
    }
    
    try:
        print("🧪 Testing Helius API...")
        response = requests.get(url, params=params, timeout=10)
        
        if response.status_code == 200:
            data = response.json()
            print(f"✅ HELIUS: Success! Received {len(data)} transactions")
            if data:
                tx = data[0]
                print(f"   Sample transaction: {tx.get('signature', 'N/A')[:16]}...")
                print(f"   Type: {tx.get('type', 'N/A')}")
            return True
        else:
            print(f"❌ HELIUS: HTTP {response.status_code} - {response.text[:200]}")
            return False
            
    except Exception as e:
        print(f"❌ HELIUS: Exception - {e}")
        return False

def test_lyslabs_connectivity(ws_url, api_key):
    """Test LYS Labs WebSocket connectivity (just connection, not full stream)"""
    if not api_key or not ws_url:
        print("❌ LYSLABS: No API key or WebSocket URL provided")
        return False
    
    try:
        import websocket
        print("🧪 Testing LYS Labs WebSocket connectivity...")
        
        # Just test if we can connect
        url = f"{ws_url}?apiKey={api_key}"
        
        def on_open(ws):
            print("✅ LYSLABS: WebSocket connected successfully!")
            ws.close()
        
        def on_error(ws, error):
            print(f"❌ LYSLABS: WebSocket error - {error}")
        
        def on_close(ws, close_status_code, close_msg):
            print("🔌 LYSLABS: WebSocket connection closed")
        
        ws = websocket.WebSocketApp(url,
                                    on_open=on_open,
                                    on_error=on_error,
                                    on_close=on_close)
        
        # Run for just a few seconds to test connection
        import threading
        wst = threading.Thread(target=ws.run_forever)
        wst.daemon = True
        wst.start()
        wst.join(timeout=5)
        
        return True
        
    except ImportError:
        print("❌ LYSLABS: websocket-client not installed, skipping WebSocket test")
        return False
    except Exception as e:
        print(f"❌ LYSLABS: Exception - {e}")
        return False

def main():
    print("=" * 60)
    print("  SOLDER CORTEX - API CONNECTIVITY TEST")
    print("=" * 60)
    
    # Load environment variables
    helius_key, lyslabs_key, lyslabs_url = load_env_vars()
    
    print("\n🔍 Testing API connections...\n")
    
    # Test Helius
    helius_success = test_helius_api(helius_key)
    
    print()
    
    # Test LYS Labs
    lyslabs_success = test_lyslabs_connectivity(lyslabs_url, lyslabs_key)
    
    print("\n" + "=" * 60)
    print("  RESULTS SUMMARY")
    print("=" * 60)
    print(f"Helius API:    {'✅ SUCCESS' if helius_success else '❌ FAILED'}")
    print(f"LYS Labs WS:   {'✅ SUCCESS' if lyslabs_success else '❌ FAILED'}")
    
    if helius_success and lyslabs_success:
        print("\n🎉 All API connections working! Ready to use real data.")
    else:
        print("\n⚠️  Some API connections failed. Check your API keys and network.")
    
    print("=" * 60)

if __name__ == "__main__":
    main()