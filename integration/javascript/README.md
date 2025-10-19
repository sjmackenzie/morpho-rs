# JavaScript Integration for morpho-rs

This directory contains JavaScript/Node.js scripts for integrating morpho-rs with AI coding tools.

## Files

### `claude_mcp_server.js`
MCP (Model Context Protocol) server for Claude Code integration.

### `lm_studio_mcp_server.js`
MCP server for LM Studio 3+ integration.

**Setup:**

1. Make the script executable:
```bash
chmod +x claude_mcp_server.js
```

2. Add to your `.claude/config.json`:
```json
{
  "mcpServers": {
    "morpho-rs": {
      "command": "/absolute/path/to/morpho-rs/integration/javascript/claude_mcp_server.js"
    }
  }
}
```

3. Start morpho-rs-agent in your Rust project:
```bash
cd /path/to/rust/project
morpho-rs-agent

# Or include multiple directories (e.g., with dependencies):
morpho-rs-agent /path/to/main/project /path/to/dependency
```

4. Restart Claude Code

**Usage:**

Once configured, you can ask Claude to analyze your Rust code:

```
"Show me the call graph for the main function"
"List all public functions in this project"
"Get the source code for generate_output"
```

Claude will automatically use morpho-rs to answer these questions.

---

### `lm_studio_mcp_server.js`
MCP server for LM Studio 3+ integration.

**Setup:**

1. Make the script executable:
```bash
chmod +x lm_studio_mcp_server.js
```

2. Add to LM Studio's MCP settings:
```json
{
  "mcpServers": {
    "morpho-rs": {
      "command": "/absolute/path/to/integration/javascript/lm_studio_mcp_server.js"
    }
  }
}
```

3. Start morpho-rs-agent in your Rust project:
```bash
cd /path/to/rust/project
morpho-rs-agent

# Or include multiple directories (e.g., with dependencies):
morpho-rs-agent /path/to/main/project /path/to/dependency
```

4. Restart or reload LM Studio

**Usage:**

Ask LM Studio to analyze your code:

```
"What functions are called by main?"
"Show me all public APIs"
"Get the source for parse_config"
```

LM Studio will use morpho-rs through MCP.

## Requirements

- Node.js 14+
- morpho-rs-agent running on `http://127.0.0.1:8080`

## Direct HTTP Usage (Node.js)

If you want to call morpho-rs from your own Node.js code:

```javascript
const http = require('http');

function callMorpho(endpoint, data) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: '127.0.0.1',
      port: 8080,
      path: endpoint,
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    };

    const req = http.request(options, (res) => {
      let body = '';
      res.on('data', chunk => body += chunk);
      res.on('end', () => resolve(JSON.parse(body)));
    });

    req.on('error', reject);
    req.write(JSON.stringify(data));
    req.end();
  });
}

// Example usage
async function main() {
  // List all public items
  const result = await callMorpho('/tool/list_all', {
    public_only: true,
    blacklist: ['target', 'tests']
  });

  console.log(result.result);
}

main();
```

## Troubleshooting

**MCP server not starting:**
- Check that the path in `.claude/config.json` is absolute
- Verify the script is executable (`chmod +x`)
- Check Claude Code logs for errors

**Connection refused:**
- Ensure `morpho-rs-agent` is running
- Verify it's listening on port 8080
- Check firewall settings

**No responses from tools:**
- Verify you're in a Rust project directory when starting the agent
- Check the agent logs for errors
- Test the HTTP endpoints directly with curl
