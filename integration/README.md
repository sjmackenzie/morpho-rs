# morpho-rs Integrations

This directory contains integration scripts for connecting morpho-rs with various AI coding tools.

## Directory Structure

```
integration/
├── python/              # Python integrations
│   ├── qwen_tools.py           # Qwen Coder integration
│   ├── lm_studio_handler.py    # LM Studio function handler
│   ├── morpho_functions.json   # LM Studio function definitions
│   └── README.md               # Python integration docs
│
└── javascript/          # JavaScript/Node.js integrations
    ├── claude_mcp_server.js    # Claude Code MCP server
    └── README.md               # JavaScript integration docs
```

## Quick Start

### Prerequisites

All integrations require:
1. **morpho-rs built and installed**
   ```bash
   cd ..
   cargo build --release
   ```

2. **morpho-rs-agent running** in your Rust project directory
   ```bash
   cd /path/to/your/rust/project
   morpho-rs-agent

   # Or specify multiple directories to analyze dependencies together:
   morpho-rs-agent /path/to/main/project /path/to/dependency1 /path/to/dependency2
   ```

### Choose Your Integration

#### [Claude Code](javascript/README.md)
Best for: Claude Code users who want seamless Rust analysis

```bash
cd javascript
chmod +x claude_mcp_server.js
# Configure in .claude/config.json
```

#### [LM Studio 3+](javascript/README.md)
Best for: LM Studio 3+ users (MCP support)

```bash
cd javascript
chmod +x lm_studio_mcp_server.js
# Configure in LM Studio MCP settings
```

**For older LM Studio** (pre-v3): See [python/README.md](python/README.md) for function calling setup.

#### [Qwen Coder](python/README.md)
Best for: Qwen Coder users or custom Python integration

```bash
cd python
pip install openai requests
python qwen_tools.py
```

## How It Works

```
┌─────────────────┐
│   AI Tool       │
│ (Claude/LM/etc) │
└────────┬────────┘
         │
         │ HTTP/MCP
         ▼
┌─────────────────┐
│  Integration    │
│     Script      │
└────────┬────────┘
         │
         │ HTTP POST
         ▼
┌─────────────────┐
│ morpho-rs-agent │
│  (port 8080)    │
└────────┬────────┘
         │
         │ Parse & Analyze
         ▼
┌─────────────────┐
│  Rust Project   │
│   (your code)   │
└─────────────────┘
```

## Available Tools

All integrations provide these three tools:

1. **list_rust_items** - List all types and functions
2. **analyze_rust_callgraph** - Generate hierarchical call graph
3. **get_rust_source** - Get formatted function source code

## Creating Custom Integrations

The morpho-rs agent exposes a simple REST API. To create your own integration:

### 1. Make HTTP POST requests to:
- `http://127.0.0.1:8080/tool/list_all`
- `http://127.0.0.1:8080/tool/generate_call_graph`
- `http://127.0.0.1:8080/tool/get_source`

### 2. Request format (JSON):
```json
{
  "public_only": false,
  "blacklist": ["target", "tests"],
  // endpoint-specific fields...
}
```

### 3. Response format (JSON):
```json
{
  "result": "...analysis output..."
}
```

See the main [README](../README.md#api-endpoints) for detailed API documentation.

## Testing Integrations

Test the agent is running:
```bash
curl -X POST http://127.0.0.1:8080/tool/list_all \
  -H "Content-Type: application/json" \
  -d '{"public_only": true}'
```

You should see JSON output with your project's public types and functions.

## Troubleshooting

### Connection Refused
- Ensure `morpho-rs-agent` is running
- Check it's using port 8080: `lsof -i :8080`
- Verify firewall settings

### Empty Results
- Make sure you're in a Rust project directory when starting the agent
- Check that the project has `.rs` files
- Try without `blacklist` first

### Integration Not Found
- Verify the integration script path is correct
- Check file permissions (scripts should be executable)
- Look for errors in the AI tool's logs

## Contributing

To add a new integration:

1. Create a directory for your language/tool
2. Add integration scripts
3. Include a README with setup instructions
4. Update this file with links to your integration
5. Submit a PR!

## License

Same as morpho-rs: MIT OR Apache-2.0
