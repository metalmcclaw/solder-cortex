const express = require('express');
const { WebSocketServer } = require('ws');
const { spawn } = require('child_process');
const path = require('path');
const http = require('http');

const app = express();
const port = 8080;

// Serve static dashboard files
app.use(express.static(path.join(__dirname, '../dashboard')));

// Documentation endpoint
app.get('/docs', (req, res) => {
    res.sendFile(path.join(__dirname, '../dashboard/README.md')); // Or render a nice page
});

const server = http.createServer(app);
const wss = new WebSocketServer({ server, path: '/ws' });

wss.on('connection', (ws) => {
    console.log('Client connected');

    // Spawn the MCP server process
    // Note: Adjust path to your binary
    const mcpProcess = spawn(path.join(__dirname, '../target/release/cortex-mcp'), [], {
        cwd: path.join(__dirname, '..') // Run from project root
    });

    mcpProcess.stdout.on('data', (data) => {
        // MCP server sends JSON-RPC responses via stdout
        const lines = data.toString().split('\n');
        for (const line of lines) {
            if (line.trim()) {
                // Forward to web client
                ws.send(line);
            }
        }
    });

    mcpProcess.stderr.on('data', (data) => {
        console.error(`MCP Error: ${data}`);
    });

    ws.on('message', (message) => {
        try {
            // Forward web client message to MCP server stdin
            // Ensure it's a string and has a newline
            const msgStr = message.toString();
            console.log('Sending to MCP:', msgStr);
            mcpProcess.stdin.write(msgStr + '\n');
        } catch (e) {
            console.error('Error handling message:', e);
        }
    });

    ws.on('close', () => {
        console.log('Client disconnected');
        mcpProcess.kill();
    });
});

server.listen(port, () => {
    console.log(`🚀 Deployment Server running at http://localhost:${port}`);
    console.log(`📂 Serving dashboard from ../dashboard`);
});
