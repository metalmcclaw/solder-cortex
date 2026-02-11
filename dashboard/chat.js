// Solder Cortex Live Chat Demo
// Auto-detect WebSocket URL based on current host
const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
const wsUrl = `${wsProtocol}//${window.location.host}/ws`;

let socket;
let msgId = 1;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 5;
const RECONNECT_DELAY = 3000;

function updateStatus(connected) {
    const statusEl = document.getElementById('connection-status');
    if (statusEl) {
        statusEl.className = connected ? 'status-connected' : 'status-disconnected';
        statusEl.textContent = connected ? '● Connected' : '○ Disconnected';
    }
}

function connect() {
    if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
        appendMessage('system', '❌ Max reconnection attempts reached. Please refresh the page.');
        return;
    }

    console.log(`Connecting to ${wsUrl}...`);
    socket = new WebSocket(wsUrl);

    socket.onopen = () => {
        console.log('Connected to MCP Server');
        reconnectAttempts = 0;
        updateStatus(true);
        appendMessage('system', '✅ Connected to Cortex MCP Server. Try: "health", "check balance", or "market trend"');
        
        // Initialize MCP protocol
        socket.send(JSON.stringify({
            jsonrpc: "2.0",
            method: "initialize",
            params: { 
                protocolVersion: "2024-11-05", 
                capabilities: {}, 
                clientInfo: { name: "web-client", version: "1.0.0" } 
            },
            id: 0
        }));
    };

    socket.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            handleResponse(data);
        } catch (e) {
            console.error('Error parsing message:', e, event.data);
        }
    };

    socket.onclose = (event) => {
        console.log('Disconnected:', event.code, event.reason);
        updateStatus(false);
        
        if (event.code !== 1000) { // Not a clean close
            reconnectAttempts++;
            appendMessage('system', `❌ Disconnected. Reconnecting (${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})...`);
            setTimeout(connect, RECONNECT_DELAY);
        }
    };

    socket.onerror = (error) => {
        console.error('WebSocket error:', error);
    };
}

function handleResponse(data) {
    if (data.result && data.result.content) {
        // Tool output
        data.result.content.forEach(item => {
            if (item.type === 'text') {
                appendMessage('agent', item.text);
            }
        });
    } else if (data.error) {
        appendMessage('error', `Error: ${data.error.message}`);
    } else if (data.id === 0 && data.result) {
        // Init response - send initialized notification
        console.log('MCP Initialized:', data.result);
        socket.send(JSON.stringify({
            jsonrpc: "2.0",
            method: "notifications/initialized"
        }));
    }
}

function appendMessage(type, text) {
    const container = document.getElementById('chat-messages');
    if (!container) return;
    
    const msgDiv = document.createElement('div');
    msgDiv.className = `message message-${type}`;
    
    // Format JSON blocks if present
    if (text.includes('{') && text.includes('}')) {
        try {
            const obj = JSON.parse(text);
            text = '<pre>' + JSON.stringify(obj, null, 2) + '</pre>';
        } catch (e) { /* Keep as text if not valid JSON */ }
    }
    
    // Convert newlines to breaks for text
    if (!text.startsWith('<pre>')) {
        text = text.replace(/\n/g, '<br>');
    }

    msgDiv.innerHTML = text;
    container.appendChild(msgDiv);
    container.scrollTop = container.scrollHeight;
}

function sendQuery(text) {
    if (!text || !socket || socket.readyState !== WebSocket.OPEN) {
        if (!socket || socket.readyState !== WebSocket.OPEN) {
            appendMessage('system', '⚠️ Not connected. Please wait for reconnection...');
        }
        return;
    }
    
    appendMessage('user', text);
    
    // Simple intent matching to map text -> tool calls
    let params = {};
    const lowerText = text.toLowerCase();
    
    if (lowerText.includes('health')) {
        params = { name: "cortex_health", arguments: {} };
    } else if (lowerText.includes('balance') || lowerText.includes('summary') || lowerText.includes('wallet')) {
        const wallet = extractWallet(text) || "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1"; // Demo default
        params = { name: "cortex_get_wallet_summary", arguments: { wallet } };
    } else if (lowerText.includes('trend') || lowerText.includes('market')) {
        params = { name: "cortex_get_market_trend", arguments: { slug: "will-solana-reach-500-in-2025", interval: "24h" } };
    } else if (lowerText.includes('pnl') || lowerText.includes('profit')) {
        const wallet = extractWallet(text) || "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
        params = { name: "cortex_get_pnl", arguments: { wallet, window: "7d" } };
    } else if (lowerText.includes('position')) {
        const wallet = extractWallet(text) || "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1";
        params = { name: "cortex_get_positions", arguments: { wallet } };
    } else if (lowerText.includes('conviction') || lowerText.includes('trader')) {
        params = { name: "cortex_detect_informed_traders", arguments: { min_conviction: 0.7 } };
    } else {
        appendMessage('system', '⚠️ Try: "health", "wallet summary", "market trend", "pnl", "positions", or "find informed traders"');
        return;
    }

    socket.send(JSON.stringify({
        jsonrpc: "2.0",
        method: "tools/call",
        params: params,
        id: msgId++
    }));
}

function extractWallet(text) {
    // Try to find a Solana wallet address (32-44 base58 chars)
    const match = text.match(/[1-9A-HJ-NP-Za-km-z]{32,44}/);
    return match ? match[0] : null;
}

document.addEventListener('DOMContentLoaded', () => {
    connect();
    
    const input = document.getElementById('chat-input');
    if (input) {
        input.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                sendQuery(input.value);
                input.value = '';
            }
        });
    }
    
    const sendBtn = document.getElementById('send-btn');
    if (sendBtn) {
        sendBtn.addEventListener('click', () => {
            sendQuery(input.value);
            input.value = '';
        });
    }
});
