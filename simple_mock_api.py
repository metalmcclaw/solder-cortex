#!/usr/bin/env python3
"""
Simple mock API server using only built-in modules
"""

from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import urllib.parse
from datetime import datetime, timezone

# Mock data
MOCK_WALLETS = {
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1": {
        "total_value_usd": 15420.50,
        "pnl": {
            "realized_24h": 234.80,
            "realized_7d": 1430.20,
            "realized_30d": 2890.45,
            "unrealized": 450.30
        },
        "risk": {
            "score": 45,
            "largest_position_pct": 23.5,
            "protocol_count": 4
        },
        "protocols": ["kamino", "raydium", "jupiter", "meteora"],
        "last_activity": "2026-02-11T02:15:00Z"
    },
    "7xKXNLgf7WrLiUxB8nQ2": {
        "total_value_usd": 8750.25,
        "pnl": {
            "realized_24h": -45.20,
            "realized_7d": 890.75,
            "realized_30d": 1560.30,
            "unrealized": -125.40
        },
        "risk": {
            "score": 32,
            "largest_position_pct": 18.9,
            "protocol_count": 3
        },
        "protocols": ["kamino", "jupiter", "orca"],
        "last_activity": "2026-02-11T01:45:00Z"
    }
}

class MockAPIHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        # Parse the URL
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        
        # Add CORS headers
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()
        
        try:
            if path == '/health':
                response = {
                    "status": "ok",
                    "version": "0.2.0-mock",
                    "message": "Mock API server providing test data"
                }
            elif path.startswith('/api/v1/user/') and path.endswith('/summary'):
                # Extract wallet from path
                wallet = path.split('/')[-2]
                if wallet in MOCK_WALLETS:
                    response = MOCK_WALLETS[wallet].copy()
                    response["wallet"] = wallet
                else:
                    response = {
                        "wallet": wallet,
                        "total_value_usd": 0,
                        "pnl": {"realized_24h": 0, "realized_7d": 0, "realized_30d": 0, "unrealized": 0},
                        "risk": {"score": 0, "largest_position_pct": 0, "protocol_count": 0},
                        "protocols": [],
                        "last_activity": datetime.now(timezone.utc).isoformat()
                    }
            elif path.startswith('/api/v1/user/') and '/pnl' in path:
                wallet = path.split('/')[4]
                if wallet in MOCK_WALLETS:
                    data = MOCK_WALLETS[wallet]
                    response = {
                        "wallet": wallet,
                        "window": "7d",
                        "total_realized": data["pnl"]["realized_7d"],
                        "total_unrealized": data["pnl"]["unrealized"],
                        "by_protocol": {
                            "kamino": {"realized": data["pnl"]["realized_7d"] * 0.4, "unrealized": data["pnl"]["unrealized"] * 0.3},
                            "raydium": {"realized": data["pnl"]["realized_7d"] * 0.3, "unrealized": data["pnl"]["unrealized"] * 0.4}
                        }
                    }
                else:
                    response = {"wallet": wallet, "total_realized": 0, "total_unrealized": 0, "by_protocol": {}}
            elif path == '/api/v1/index':
                response = {
                    "subscriptions": [
                        {
                            "wallet": "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
                            "started_at": "2026-02-10T08:30:00Z",
                            "transactions_processed": 847,
                            "running": True
                        }
                    ]
                }
            else:
                response = {"error": "Endpoint not found", "path": path}
            
            self.wfile.write(json.dumps(response, indent=2).encode())
            
        except Exception as e:
            error_response = {"error": str(e)}
            self.wfile.write(json.dumps(error_response).encode())
    
    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS')
        self.send_header('Access-Control-Allow-Headers', 'Content-Type')
        self.end_headers()
    
    def log_message(self, format, *args):
        # Reduce verbosity
        pass

if __name__ == '__main__':
    server = HTTPServer(('0.0.0.0', 3001), MockAPIHandler)
    print("🤘 Solder Cortex Mock API Server")
    print("📊 Serving realistic test data for dashboard")
    print("🌐 http://localhost:3001")
    print("✅ Ready for API requests...")
    server.serve_forever()