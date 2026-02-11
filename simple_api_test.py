#!/usr/bin/env python3
"""
Minimal API test to verify Helius API works and to demonstrate the API connections.
"""
import requests
import json

# Read API keys from .env file
def load_keys():
    keys = {}
    with open('.env', 'r') as f:
        for line in f:
            if '=' in line and not line.startswith('#'):
                key, value = line.strip().split('=', 1)
                keys[key] = value
    return keys

def test_helius_simple():
    """Test Helius API with a simple request"""
    keys = load_keys()
    api_key = keys.get('CORTEX__HELIUS__API_KEY')
    
    if not api_key:
        print("❌ No Helius API key found")
        return False
    
    print(f"🔑 Using Helius API key: {api_key[:8]}...{api_key[-4:]}")
    
    # Use Jupiter's well-known wallet for testing
    wallet = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"
    url = f"https://api.helius.xyz/v0/addresses/{wallet}/transactions"
    
    params = {
        'api-key': api_key,
        'limit': 3
    }
    
    print(f"🧪 Testing Helius API with wallet: {wallet[:16]}...")
    
    try:
        response = requests.get(url, params=params, timeout=10)
        
        if response.status_code == 200:
            data = response.json()
            print(f"✅ SUCCESS: Received {len(data)} transactions from Helius")
            
            for i, tx in enumerate(data[:3]):  # Show first 3
                print(f"  [{i+1}] {tx['signature'][:16]}... | Type: {tx.get('type', 'UNKNOWN')} | Fee: {tx.get('fee', 0)}")
                if tx.get('tokenTransfers'):
                    print(f"      Token transfers: {len(tx['tokenTransfers'])}")
                if tx.get('events') and tx['events'].get('swap'):
                    swap = tx['events']['swap']
                    print(f"      Swap detected! Inputs: {len(swap.get('tokenInputs', []))} Outputs: {len(swap.get('tokenOutputs', []))}")
            
            return True
        else:
            print(f"❌ FAILED: HTTP {response.status_code}")
            print(f"Response: {response.text[:300]}...")
            return False
            
    except Exception as e:
        print(f"❌ ERROR: {e}")
        return False

def main():
    print("=" * 60)
    print("  SOLDER CORTEX - REAL API DATA TEST")
    print("=" * 60)
    print()
    
    if test_helius_simple():
        print("\n🎉 Real Helius API data is working!")
        print("✅ This confirms the API keys are valid and the system can fetch real transaction data.")
        print("✅ The indexer should now work with real data instead of mock data.")
    else:
        print("\n❌ Helius API test failed!")
        print("⚠️  Check your API key and network connection.")

if __name__ == "__main__":
    main()