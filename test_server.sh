#!/bin/bash

# Simple test script for ADB MCP Server

# Start server in background
echo "[*] Starting ADB MCP Server..."
./target/release/adb-mcp &
SERVER_PID=$!

# Give server time to start
sleep 1

# Test 1: Initialize
echo "[*] Test 1: Sending initialize request..."
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}' | nc localhost 9999 2>/dev/null || \
  timeout 1 ./target/release/adb-mcp <<< '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}' | head -1

# Test 2: List Tools
echo "[*] Test 2: Listing tools..."
timeout 1 ./target/release/adb-mcp <<< '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | head -1

# Clean up
kill $SERVER_PID 2>/dev/null
wait $SERVER_PID 2>/dev/null

echo "[*] Tests complete"
