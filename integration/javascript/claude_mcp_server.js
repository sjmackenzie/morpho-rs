#!/usr/bin/env node

/**
 * MCP (Model Context Protocol) server for Claude Code
 *
 * This server allows Claude Code to analyze Rust projects using morpho-rs.
 *
 * Setup:
 *   1. Make this file executable: chmod +x claude_mcp_server.js
 *   2. Add to .claude/config.json:
 *      {
 *        "mcpServers": {
 *          "morpho-rs": {
 *            "command": "/path/to/morpho-rs/integration/javascript/claude_mcp_server.js"
 *          }
 *        }
 *      }
 *   3. Start morpho-rs-agent in your Rust project directory
 *   4. Restart Claude Code
 *
 * Usage:
 *   Ask Claude to analyze your Rust code and it will automatically use morpho-rs
 */

const http = require('http');
const { spawn } = require('child_process');

// Start morpho-rs-agent
const agent = spawn('./target/release/morpho-rs-agent');

agent.stdout.on('data', (data) => console.error(`Agent: ${data}`));
agent.stderr.on('data', (data) => console.error(`Agent Error: ${data}`));

// MCP Server implementation
const server = http.createServer(async (req, res) => {
  if (req.method === 'POST' && req.url === '/mcp') {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', async () => {
      const mcpRequest = JSON.parse(body);

      // Handle get_project_info (GET request)
      if (mcpRequest.method === 'tools/call' && mcpRequest.params.name === 'get_project_info') {
        const infoOptions = {
          hostname: '127.0.0.1',
          port: 8080,
          path: '/info',
          method: 'GET'
        };

        const infoReq = http.request(infoOptions, (infoRes) => {
          let data = '';
          infoRes.on('data', chunk => data += chunk);
          infoRes.on('end', () => {
            try {
              const infoData = JSON.parse(data);
              let text = `Primary Project: ${infoData.primary_project.name} (${infoData.primary_project.path})\n`;
              if (infoData.dependencies.length > 0) {
                text += "\nDependencies:\n";
                for (const dep of infoData.dependencies) {
                  text += `  - ${dep.name} (${dep.path})\n`;
                }
              }

              res.writeHead(200, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({
                jsonrpc: '2.0',
                id: mcpRequest.id,
                result: { content: [{ type: 'text', text: text }] }
              }));
            } catch (e) {
              res.writeHead(200, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({
                jsonrpc: '2.0',
                id: mcpRequest.id,
                error: { code: -32603, message: `Failed to parse /info response: ${e.message}` }
              }));
            }
          });
        });

        infoReq.on('error', (err) => {
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({
            jsonrpc: '2.0',
            id: mcpRequest.id,
            error: { code: -32603, message: err.message }
          }));
        });

        infoReq.end();
        return;
      }

      // Map MCP tool calls to morpho-rs agent
      let morphoEndpoint, morphoBody;

      if (mcpRequest.method === 'tools/call' && mcpRequest.params.name === 'list_rust_items') {
        morphoEndpoint = '/tool/list_all';
        morphoBody = JSON.stringify({
          public_only: mcpRequest.params.arguments.public_only || false,
          blacklist: mcpRequest.params.arguments.blacklist || []
        });
      } else if (mcpRequest.method === 'tools/call' && mcpRequest.params.name === 'analyze_rust_callgraph') {
        morphoEndpoint = '/tool/generate_call_graph';
        morphoBody = JSON.stringify({
          root_function: mcpRequest.params.arguments.function,
          public_only: mcpRequest.params.arguments.public_only || false,
          blacklist: mcpRequest.params.arguments.blacklist || []
        });
      } else if (mcpRequest.method === 'tools/call' && mcpRequest.params.name === 'get_rust_source') {
        morphoEndpoint = '/tool/get_source';
        morphoBody = JSON.stringify({
          function: mcpRequest.params.arguments.function,
          blacklist: mcpRequest.params.arguments.blacklist || []
        });
      }

      // Forward to morpho-rs-agent
      const options = {
        hostname: '127.0.0.1',
        port: 8080,
        path: morphoEndpoint,
        method: 'POST',
        headers: { 'Content-Type': 'application/json' }
      };

      const agentReq = http.request(options, (agentRes) => {
        let data = '';
        agentRes.on('data', chunk => data += chunk);
        agentRes.on('end', () => {
          try {
            const response = JSON.parse(data);
            if (response.error) {
              res.writeHead(200, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({
                jsonrpc: '2.0',
                id: mcpRequest.id,
                error: { code: -32603, message: response.error }
              }));
            } else {
              res.writeHead(200, { 'Content-Type': 'application/json' });
              res.end(JSON.stringify({
                jsonrpc: '2.0',
                id: mcpRequest.id,
                result: { content: [{ type: 'text', text: response.result }] }
              }));
            }
          } catch (e) {
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify({
              jsonrpc: '2.0',
              id: mcpRequest.id,
              error: { code: -32603, message: `Failed to parse response: ${e.message}. Body: ${data}` }
            }));
          }
        });
      });

      agentReq.write(morphoBody);
      agentReq.end();
    });
  }
});

server.listen(3000, () => {
  console.log('MCP Server listening on port 3000');
});
