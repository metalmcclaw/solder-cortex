const express = require('express');
const { WebSocketServer } = require('ws');
const { spawn } = require('child_process');
const path = require('path');
const http = require('http');

const app = express();
const port = process.env.PORT || 8080;

// MCP binary path - configurable via env
const mcpBinaryPath = process.env.MCP_BINARY_PATH || path.join(__dirname, '../target/release/cortex-mcp');

// Serve static dashboard files
const dashboardPath = process.env.DASHBOARD_PATH || path.join(__dirname, '../dashboard');
app.use(express.static(dashboardPath));

// Health check endpoint
app.get('/health', (req, res) => {
    res.json({ status: 'ok', version: '0.2.0' });
});

// Proxy to internal Rust API
app.use('/api', async (req, res) => {
    try {
        const cortexUrl = process.env.CORTEX_API_URL || 'http://cortex:3000';
        const proxyUrl = `${cortexUrl}${req.originalUrl}`;
        
        const fetch = await import('node-fetch');
        const response = await fetch.default(proxyUrl);
        const data = await response.json();
        
        res.json(data);
    } catch (error) {
        console.error('Proxy error:', error);
        res.status(500).json({ error: 'Backend not available' });
    }
});

const server = http.createServer(app);
const wss = new WebSocketServer({ server, path: '/ws' });

wss.on('connection', (ws, req) => {
    const clientIp = req.headers['x-forwarded-for'] || req.socket.remoteAddress;
    console.log(`Client connected from ${clientIp}`);

    // Spawn the MCP server process
    const mcpProcess = spawn(mcpBinaryPath, [], {
        cwd: process.cwd(),
        env: {
            ...process.env,
            CORTEX_DEMO_MODE: process.env.CORTEX_DEMO_MODE || "true",
            CORTEX_API_URL: process.env.CORTEX_API_URL || "http://localhost:3000",
            DATABASE_URL: process.env.DATABASE_URL,
            CLICKHOUSE_URL: process.env.CLICKHOUSE_URL
        }
    });

    let mcpAlive = true;

    mcpProcess.on('error', (err) => {
        console.error('Failed to start MCP process:', err);
        mcpAlive = false;
        if (ws.readyState === ws.OPEN) {
            ws.send(JSON.stringify({
                jsonrpc: "2.0",
                error: { code: -32000, message: "Server Error: Failed to start MCP process" },
                id: null
            }));
            ws.close();
        }
    });

    mcpProcess.on('exit', (code, signal) => {
        console.log(`MCP process exited with code ${code}, signal ${signal}`);
        mcpAlive = false;
        if (ws.readyState === ws.OPEN) {
            ws.close();
        }
    });

    // Buffer for incomplete JSON lines
    let stdoutBuffer = '';

    mcpProcess.stdout.on('data', (data) => {
        stdoutBuffer += data.toString();
        
        // Process complete lines
        const lines = stdoutBuffer.split('\n');
        stdoutBuffer = lines.pop() || ''; // Keep incomplete line in buffer
        
        for (const line of lines) {
            const trimmed = line.trim();
            if (trimmed && trimmed.startsWith('{')) {
                try {
                    // Validate JSON before sending
                    JSON.parse(trimmed);
                    ws.send(trimmed);
                } catch (e) {
                    console.error('Invalid JSON from MCP:', trimmed);
                }
            }
        }
    });

    mcpProcess.stderr.on('data', (data) => {
        console.error(`MCP stderr: ${data}`);
    });

    ws.on('message', (message) => {
        if (!mcpAlive) {
            ws.send(JSON.stringify({
                jsonrpc: "2.0",
                error: { code: -32000, message: "MCP process not running" },
                id: null
            }));
            return;
        }

        try {
            const msgStr = message.toString();
            console.log('→ MCP:', msgStr.substring(0, 100) + (msgStr.length > 100 ? '...' : ''));
            mcpProcess.stdin.write(msgStr + '\n');
        } catch (e) {
            console.error('Error sending to MCP:', e);
        }
    });

    ws.on('close', () => {
        console.log('Client disconnected');
        if (mcpAlive) {
            mcpProcess.kill('SIGTERM');
        }
    });

    ws.on('error', (err) => {
        console.error('WebSocket error:', err);
        if (mcpAlive) {
            mcpProcess.kill('SIGTERM');
        }
    });
});

server.listen(port, '0.0.0.0', () => {
    console.log(`🚀 Solder Cortex Server running on port ${port}`);
    console.log(`📂 Dashboard: ${dashboardPath}`);
    console.log(`🔧 MCP Binary: ${mcpBinaryPath}`);
});
