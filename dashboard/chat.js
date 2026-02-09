// Solder Cortex Live Chat Demo
const wsUrl = (window.location.protocol === 'https:' ? 'wss://' : 'ws://') + window.location.host + '/ws';
let socket;
let msgId = 1;

function connect() {
    socket = new WebSocket(wsUrl);

    socket.onopen = () => {
        console.log('Connected to MCP Server');
        appendMessage('system', '✅ Connected to Cortex MCP Server. Ready for queries.');
        
        // Initialize
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
            console.error('Error parsing message:', e);
        }
    };

    socket.onclose = () => {
        console.log('Disconnected');
        appendMessage('system', '❌ Disconnected from server. Reconnecting...');
        setTimeout(connect, 3000);
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
    } else if (data.id === 0) {
        // Init response
        console.log('Initialized:', data);
        socket.send(JSON.stringify({
            jsonrpc: "2.0",
            method: "notifications/initialized"
        }));
    }
}

function appendMessage(type, text) {
    const container = document.getElementById('chat-messages');
    const msgDiv = document.createElement('div');
    msgDiv.className = `message message-${type}`;
    
    // Format JSON blocks if present
    if (text.includes('{') && text.includes('}')) {
        try {
            // Try to format if it looks like JSON
            const obj = JSON.parse(text);
            text = '<pre>' + JSON.stringify(obj, null, 2) + '</pre>';
        } catch (e) { } // Keep as text if not valid JSON
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
    if (!text) return;
    
    appendMessage('user', text);
    
    // Simple intent matching to map text -> tool calls (like the python script)
    let method = "tools/call";
    let params = {};
    
    if (text.toLowerCase().includes('health')) {
        params = { name: "cortex_health", arguments: {} };
    } else if (text.toLowerCase().includes('balance') || text.toLowerCase().includes('summary')) {
        const wallet = "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1"; // Demo
        params = { name: "cortex_get_wallet_summary", arguments: { wallet } };
    } else if (text.toLowerCase().includes('trend') || text.toLowerCase().includes('market')) {
        params = { name: "cortex_get_market_trend", arguments: { slug: "will-solana-reach-500-in-2025", interval: "24h" } };
    } else {
        // Default fallthrough or raw JSON if user knows schema
        appendMessage('system', '⚠️ I am a simple demo. Try "health", "check balance", or "market trend".');
        return;
    }

    socket.send(JSON.stringify({
        jsonrpc: "2.0",
        method: method,
        params: params,
        id: msgId++
    }));
}

document.addEventListener('DOMContentLoaded', () => {
    connect();
    
    const input = document.getElementById('chat-input');
    input.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            sendQuery(input.value);
            input.value = '';
        }
    });
    
    document.getElementById('send-btn').addEventListener('click', () => {
        sendQuery(input.value);
        input.value = '';
    });
});
