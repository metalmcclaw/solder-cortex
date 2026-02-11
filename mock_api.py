#!/usr/bin/env python3
"""
Mock API server for Solder Cortex dashboard
Provides realistic test data when backend is not available
"""

from flask import Flask, jsonify
from flask_cors import CORS
import random
import time
from datetime import datetime, timezone

app = Flask(__name__)
CORS(app)

# Mock wallet data
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
    },
    "95n9a8ydvtwN": {
        "total_value_usd": 22100.80,
        "pnl": {
            "realized_24h": 567.90,
            "realized_7d": 2340.60,
            "realized_30d": 4560.80,
            "unrealized": 780.20
        },
        "risk": {
            "score": 68,
            "largest_position_pct": 35.2,
            "protocol_count": 6
        },
        "protocols": ["kamino", "raydium", "jupiter", "meteora", "orca", "marinade"],
        "last_activity": "2026-02-11T02:20:00Z"
    }
}

@app.route('/health')
def health():
    return jsonify({
        "status": "ok",
        "version": "0.2.0-mock",
        "message": "Mock API server for development"
    })

@app.route('/api/v1/user/<wallet>/summary')
def get_wallet_summary(wallet):
    if wallet in MOCK_WALLETS:
        data = MOCK_WALLETS[wallet].copy()
        data["wallet"] = wallet
        return jsonify(data)
    else:
        # Return placeholder for unknown wallets
        return jsonify({
            "wallet": wallet,
            "total_value_usd": 0,
            "pnl": {
                "realized_24h": 0,
                "realized_7d": 0,
                "realized_30d": 0,
                "unrealized": 0
            },
            "risk": {
                "score": 0,
                "largest_position_pct": 0,
                "protocol_count": 0
            },
            "protocols": [],
            "last_activity": datetime.now(timezone.utc).isoformat()
        })

@app.route('/api/v1/user/<wallet>/pnl')
def get_wallet_pnl(wallet):
    if wallet in MOCK_WALLETS:
        data = MOCK_WALLETS[wallet]
        return jsonify({
            "wallet": wallet,
            "window": "7d",
            "total_realized": data["pnl"]["realized_7d"],
            "total_unrealized": data["pnl"]["unrealized"],
            "by_protocol": {
                "kamino": {"realized": data["pnl"]["realized_7d"] * 0.4, "unrealized": data["pnl"]["unrealized"] * 0.3},
                "raydium": {"realized": data["pnl"]["realized_7d"] * 0.3, "unrealized": data["pnl"]["unrealized"] * 0.4},
                "jupiter": {"realized": data["pnl"]["realized_7d"] * 0.2, "unrealized": data["pnl"]["unrealized"] * 0.2},
                "orca": {"realized": data["pnl"]["realized_7d"] * 0.1, "unrealized": data["pnl"]["unrealized"] * 0.1}
            }
        })
    else:
        return jsonify({
            "wallet": wallet,
            "window": "7d", 
            "total_realized": 0,
            "total_unrealized": 0,
            "by_protocol": {}
        })

@app.route('/api/v1/user/<wallet>/positions')
def get_wallet_positions(wallet):
    if wallet in MOCK_WALLETS:
        total_value = MOCK_WALLETS[wallet]["total_value_usd"]
        return jsonify({
            "wallet": wallet,
            "total_value_usd": total_value,
            "positions": [
                {
                    "protocol": "kamino",
                    "type": "supply",
                    "token": "SOL",
                    "amount": "50.5",
                    "usd_value": str(total_value * 0.4),
                    "apy": "5.34"
                },
                {
                    "protocol": "raydium", 
                    "type": "lp",
                    "token": "SOL/USDC",
                    "amount": "LP-1234",
                    "usd_value": str(total_value * 0.3),
                    "apy": "12.8"
                },
                {
                    "protocol": "jupiter",
                    "type": "stake",
                    "token": "JUP",
                    "amount": "2500",
                    "usd_value": str(total_value * 0.2),
                    "apy": "8.2"
                },
                {
                    "protocol": "meteora",
                    "type": "farm",
                    "token": "BONK",
                    "amount": "5200000", 
                    "usd_value": str(total_value * 0.1),
                    "apy": "24.7"
                }
            ]
        })
    else:
        return jsonify({
            "wallet": wallet,
            "total_value_usd": 0,
            "positions": []
        })

@app.route('/api/v1/index', methods=['GET'])
def list_subscriptions():
    return jsonify({
        "subscriptions": [
            {
                "wallet": "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
                "started_at": "2026-02-10T08:30:00Z",
                "transactions_processed": 847,
                "running": True
            },
            {
                "wallet": "7xKXNLgf7WrLiUxB8nQ2", 
                "started_at": "2026-02-10T10:15:00Z",
                "transactions_processed": 623,
                "running": True
            },
            {
                "wallet": "95n9a8ydvtwN",
                "started_at": "2026-02-10T12:45:00Z", 
                "transactions_processed": 1205,
                "running": True
            }
        ]
    })

if __name__ == '__main__':
    print("🤘 Starting Solder Cortex Mock API Server")
    print("📊 Serving realistic test data for dashboard development")
    print("🌐 Server running on http://localhost:3001")
    app.run(host='0.0.0.0', port=3001, debug=True)