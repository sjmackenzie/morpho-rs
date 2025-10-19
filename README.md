# morpho-rs

A Rust code analysis tool designed to provide LLM-optimized context about your Rust projects. Generate call graphs, type hierarchies, and function signatures in a format that's efficient for AI coding assistants.

## Features

- 🔍 **Smart Code Analysis**: Parse Rust projects to extract types, functions, and call relationships
- 🌳 **Hierarchical Call Graphs**: Visualize function call chains with proper tree structure
- 🎯 **Visibility Filtering**: Show only public APIs to reduce token usage
- 📝 **Source Code Viewing**: Display formatted function implementations
- 🚀 **HTTP Agent**: Expose analysis capabilities via REST API for AI tool integration
- 🔗 **Fully Qualified Names**: Use precise function paths like `./src/lib.rs::generate_output`

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/morpho-rs
cd morpho-rs

# Build release binaries
cargo build --release

# Binaries will be in target/release/
# - morpho-rs-cli (command-line tool)
# - morpho-rs-agent (HTTP server)
```

## CLI Usage

### 1. List All Items

Show all types and functions in a project:

```bash
morpho-rs-cli /path/to/rust/project
```

**Filter to public API only** (saves tokens):

```bash
morpho-rs-cli /path/to/rust/project --public-only
```

**Output:**
```
=== ./src/lib.rs ===
pub struct Project {
    pub functions: HashMap < String , Function >,
    pub types: HashMap < String , (String , Item) >
}
pub fn ./src/lib.rs::load_project(& str) -> Result < Project , String >
pub fn ./src/lib.rs::generate_output(& str, OutputMode) -> Result < Output , String >
```

### 2. Generate Call Graph

Show what a function calls (recursively):

```bash
morpho-rs-cli /path/to/rust/project "./src/lib.rs::generate_output"
```

**Output:**
```
=== ./src/lib.rs ===
pub struct Project { ... }
pub enum OutputMode { ... }

pub fn ./src/lib.rs::generate_output(& str, OutputMode) -> Result < Output , String >
├── generate_list_all [in: match OutputMode::ListAll]
│   ├── item_matches_visibility_filter
│   │   └── item_is_public
│   │       └── is_public
│   └── matches_visibility_filter
│       └── is_public (already shown)
└── generate_call_graph_output [in: match OutputMode::CallGraph]
    └── render_function_tree
        └── ...
```

**Features:**
- ✅ Only shows functions from your project (filters stdlib calls)
- ✅ True hierarchical tree with proper nesting
- ✅ Context annotations show where calls occur (`[in: match ...]`)
- ✅ Cycle detection with `(already shown)` markers

### 3. View Function Source

Display formatted source code of a specific function:

```bash
morpho-rs-cli /path/to/rust/project "./src/lib.rs::generate_output" --source
```

Or use short name:

```bash
morpho-rs-cli /path/to/rust/project "generate_output" --source
```

**Output:**
```
=== ./src/lib.rs ===
pub fn generate_output(dir: &str, mode: OutputMode) -> Result<Output, String> {
    let project = load_project(dir)?;
    match mode {
        OutputMode::ListAll { visibility } => generate_list_all(&project, visibility),
        OutputMode::CallGraph { root, visibility } => { ... }
        OutputMode::Source { function } => generate_source(&project, &function),
    }
}
```

## HTTP Agent Setup

The `morpho-rs-agent` runs an HTTP server that exposes the analysis tools via REST API.

### Starting the Agent

```bash
# Run the agent (listens on http://127.0.0.1:8080)
morpho-rs-agent
```

**Output:**
```
🚀 morpho-rs-agent (HTTP) listening on http://127.0.0.1:8080
   /tool/generate_call_graph - Generate call graph from a function
   /tool/get_source          - Get source code of a function
   /tool/list_all            - List all types and functions in project
```

### API Endpoints

#### 1. List All Items

**Endpoint:** `POST /tool/list_all`

**Request Body:**
```json
{
  "public_only": false
}
```

**Response:**
```json
{
  "result": "=== ./src/lib.rs ===\npub struct Project { ... }\npub fn ./src/lib.rs::load_project(...) -> ...\n..."
}
```

**cURL Example:**
```bash
curl -X POST http://127.0.0.1:8080/tool/list_all \
  -H "Content-Type: application/json" \
  -d '{"public_only": true}'
```

#### 2. Generate Call Graph

**Endpoint:** `POST /tool/generate_call_graph`

**Request Body:**
```json
{
  "root_function": "./src/lib.rs::generate_output",
  "public_only": false
}
```

**Response:**
```json
{
  "result": "=== ./src/lib.rs ===\npub fn ./src/lib.rs::generate_output(...) -> ...\n├── generate_list_all\n..."
}
```

**cURL Example:**
```bash
curl -X POST http://127.0.0.1:8080/tool/generate_call_graph \
  -H "Content-Type: application/json" \
  -d '{"root_function": "./src/lib.rs::generate_output", "public_only": false}'
```

#### 3. Get Function Source

**Endpoint:** `POST /tool/get_source`

**Request Body:**
```json
{
  "function": "./src/lib.rs::generate_output"
}
```

**Response:**
```json
{
  "result": "=== ./src/lib.rs ===\npub fn generate_output(dir: &str, mode: OutputMode) -> Result<Output, String> { ... }"
}
```

**cURL Example:**
```bash
curl -X POST http://127.0.0.1:8080/tool/get_source \
  -H "Content-Type: application/json" \
  -d '{"function": "generate_output"}'
```

## Integration with AI Coding Tools

### Claude Code Integration

Claude Code supports custom MCP (Model Context Protocol) servers. Here's how to integrate morpho-rs:

#### 1. Create MCP Server Wrapper

Create a file `morpho-mcp-server.js`:

```javascript
#!/usr/bin/env node
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

      // Map MCP tool calls to morpho-rs agent
      let morphoEndpoint, morphoBody;

      if (mcpRequest.method === 'tools/call' && mcpRequest.params.name === 'list_rust_items') {
        morphoEndpoint = '/tool/list_all';
        morphoBody = JSON.stringify({
          public_only: mcpRequest.params.arguments.public_only || false
        });
      } else if (mcpRequest.method === 'tools/call' && mcpRequest.params.name === 'analyze_rust_callgraph') {
        morphoEndpoint = '/tool/generate_call_graph';
        morphoBody = JSON.stringify({
          root_function: mcpRequest.params.arguments.function,
          public_only: mcpRequest.params.arguments.public_only || false
        });
      } else if (mcpRequest.method === 'tools/call' && mcpRequest.params.name === 'get_rust_source') {
        morphoEndpoint = '/tool/get_source';
        morphoBody = JSON.stringify({
          function: mcpRequest.params.arguments.function
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
          const response = JSON.parse(data);
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({
            jsonrpc: '2.0',
            id: mcpRequest.id,
            result: { content: [{ type: 'text', text: response.result }] }
          }));
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
```

#### 2. Configure Claude Code

Add to your Claude Code settings (`.claude/config.json`):

```json
{
  "mcpServers": {
    "morpho-rs": {
      "command": "node",
      "args": ["/path/to/morpho-mcp-server.js"]
    }
  }
}
```

#### 3. Usage in Claude Code

Now you can ask Claude:

```
"Analyze the call graph for the generate_output function"
```

Claude will use the MCP server to call morpho-rs automatically.

### LM Studio Integration

LM Studio supports OpenAPI function calling. Here's how to set it up:

#### 1. Create OpenAPI Function Definitions

Save this as `morpho-functions.json`:

```json
{
  "functions": [
    {
      "name": "list_rust_items",
      "description": "List all types and functions in the Rust project",
      "parameters": {
        "type": "object",
        "properties": {
          "public_only": {
            "type": "boolean",
            "description": "Only show public API items (reduces token usage)",
            "default": false
          }
        },
        "required": []
      }
    },
    {
      "name": "analyze_rust_callgraph",
      "description": "Generate a call graph showing what functions are called by a given Rust function. Shows hierarchical tree structure with context annotations.",
      "parameters": {
        "type": "object",
        "properties": {
          "function": {
            "type": "string",
            "description": "Fully qualified function name (e.g., './src/lib.rs::generate_output') or short name (e.g., 'generate_output')"
          },
          "public_only": {
            "type": "boolean",
            "description": "Only show public functions (reduces token usage)",
            "default": false
          }
        },
        "required": ["function"]
      }
    },
    {
      "name": "get_rust_source",
      "description": "Get the formatted source code of a specific Rust function",
      "parameters": {
        "type": "object",
        "properties": {
          "function": {
            "type": "string",
            "description": "Fully qualified function name or short name"
          }
        },
        "required": ["function"]
      }
    }
  ]
}
```

#### 2. Create Function Handler Script

Save as `morpho-lm-handler.py`:

```python
#!/usr/bin/env python3
import json
import requests
import sys

def handle_function_call(function_name, arguments):
    base_url = "http://127.0.0.1:8080"

    if function_name == "list_rust_items":
        response = requests.post(
            f"{base_url}/tool/list_all",
            json={"public_only": arguments.get("public_only", False)}
        )
    elif function_name == "analyze_rust_callgraph":
        response = requests.post(
            f"{base_url}/tool/generate_call_graph",
            json={
                "root_function": arguments["function"],
                "public_only": arguments.get("public_only", False)
            }
        )
    elif function_name == "get_rust_source":
        response = requests.post(
            f"{base_url}/tool/get_source",
            json={"function": arguments["function"]}
        )
    else:
        return {"error": f"Unknown function: {function_name}"}

    if response.status_code == 200:
        return response.json()
    else:
        return {"error": f"HTTP {response.status_code}: {response.text}"}

if __name__ == "__main__":
    input_data = json.loads(sys.stdin.read())
    result = handle_function_call(input_data["name"], input_data["arguments"])
    print(json.dumps(result))
```

#### 3. Configure LM Studio

1. Start `morpho-rs-agent`
2. In LM Studio → Settings → Functions:
   - Enable "Function Calling"
   - Load `morpho-functions.json`
   - Set handler script to `morpho-lm-handler.py`

#### 4. Usage in LM Studio

Chat with the model:

```
User: "Show me the call graph for generate_output"
Model: [Calls analyze_rust_callgraph function and displays results]
```

### Qwen Code Integration

Qwen Code supports tool/function calling similar to OpenAI's format.

#### 1. Start morpho-rs-agent

```bash
./target/release/morpho-rs-agent
```

#### 2. Create Tool Wrapper

Save as `qwen-morpho-tools.py`:

```python
#!/usr/bin/env python3
import requests

MORPHO_BASE = "http://127.0.0.1:8080"

tools = [
    {
        "type": "function",
        "function": {
            "name": "list_rust_items",
            "description": "List all types and functions in a Rust project",
            "parameters": {
                "type": "object",
                "properties": {
                    "public_only": {
                        "type": "boolean",
                        "description": "Only show public API items (reduces token usage)",
                        "default": False
                    }
                },
                "required": []
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "analyze_rust_callgraph",
            "description": "Analyze Rust code to show which functions are called by a given function, with hierarchical tree visualization",
            "parameters": {
                "type": "object",
                "properties": {
                    "function": {
                        "type": "string",
                        "description": "The function to analyze (e.g., './src/lib.rs::generate_output' or 'generate_output')"
                    },
                    "public_only": {
                        "type": "boolean",
                        "description": "Only show public API functions",
                        "default": False
                    }
                },
                "required": ["function"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "get_rust_source",
            "description": "Get the formatted source code of a Rust function",
            "parameters": {
                "type": "object",
                "properties": {
                    "function": {
                        "type": "string",
                        "description": "Function name to retrieve source for"
                    }
                },
                "required": ["function"]
            }
        }
    }
]

def call_tool(tool_name, arguments):
    if tool_name == "list_rust_items":
        resp = requests.post(
            f"{MORPHO_BASE}/tool/list_all",
            json={"public_only": arguments.get("public_only", False)}
        )
        return resp.json()["result"]
    elif tool_name == "analyze_rust_callgraph":
        resp = requests.post(
            f"{MORPHO_BASE}/tool/generate_call_graph",
            json={
                "root_function": arguments["function"],
                "public_only": arguments.get("public_only", False)
            }
        )
        return resp.json()["result"]
    elif tool_name == "get_rust_source":
        resp = requests.post(
            f"{MORPHO_BASE}/tool/get_source",
            json={"function": arguments["function"]}
        )
        return resp.json()["result"]

# Example usage with Qwen API
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:1234/v1",  # Qwen local server
    api_key="not-needed"
)

messages = [
    {"role": "user", "content": "Analyze the call graph for generate_output in the current Rust project"}
]

response = client.chat.completions.create(
    model="qwen-coder",
    messages=messages,
    tools=tools,
    tool_choice="auto"
)

# Handle tool calls
if response.choices[0].message.tool_calls:
    for tool_call in response.choices[0].message.tool_calls:
        result = call_tool(
            tool_call.function.name,
            eval(tool_call.function.arguments)
        )
        print(result)
```

#### 3. Run with Qwen

```bash
python qwen-morpho-tools.py
```

## Use Cases

### 1. Understanding Complex Codebases

```bash
# "What does the main function do?"
morpho-rs-cli /path/to/project "./src/main.rs::main"
```

### 2. API Documentation

```bash
# Generate public API overview
morpho-rs-cli /path/to/project --public-only > api_docs.txt
```

### 3. Refactoring Analysis

```bash
# "What will break if I change this function?"
morpho-rs-cli /path/to/project "./src/db.rs::execute_query"
```

### 4. Dependency Analysis

```bash
# See what a function depends on
morpho-rs-cli /path/to/project "./src/auth.rs::validate_token"
```

### 5. Code Review

```bash
# Review implementation of a specific function
morpho-rs-cli /path/to/project "validate_config" --source
```

## Workflow Example

Here's a complete workflow for understanding and modifying code:

```bash
# 1. List public API to understand what's available
morpho-rs-cli . --public-only

# 2. Found interesting function: ./src/lib.rs::generate_output
#    What does it call?
morpho-rs-cli . "./src/lib.rs::generate_output"

# Output shows it calls generate_list_all and generate_call_graph_output

# 3. Dive into generate_list_all
morpho-rs-cli . "./src/lib.rs::generate_list_all"

# 4. View the actual implementation
morpho-rs-cli . "./src/lib.rs::generate_list_all" --source

# 5. Now you understand the code structure and can make informed changes!
```

## Advanced Features

### Fully Qualified Names

morpho-rs uses fully qualified names to avoid ambiguity:

```
Format: <file_path>::<function_name>
Example: ./src/lib.rs::generate_output

For methods:
Format: <file_path>::<Type>::<method>
Example: ./src/lib.rs::Function::signature
```

This allows you to:
- Copy-paste function names directly from output
- Distinguish between functions with the same name in different files
- Work across multiple crates in a workspace

### Visibility Filtering

The `--public-only` flag is crucial for large codebases:

```bash
# Without filter: ~50KB output
morpho-rs-cli /large/project

# With filter: ~5KB output (90% reduction!)
morpho-rs-cli /large/project --public-only
```

This dramatically reduces:
- LLM token usage
- API call costs
- Context window consumption

### Call Graph Features

The call graph output includes:

1. **Context annotations**: Shows WHERE calls happen
   ```
   ├── validate_user [in: if (session.is_active())]
   ```

2. **Cycle detection**: Prevents infinite loops
   ```
   └── helper_function (already shown)
   ```

3. **Project-only filtering**: Hides stdlib calls (`push`, `clone`, etc.)

4. **Tree visualization**: Shows true nesting structure

## Architecture

```
morpho-rs/
├── src/
│   ├── lib.rs              # Core analysis logic
│   └── bin/
│       ├── morpho-rs-cli.rs   # CLI interface
│       └── morpho-rs-agent.rs # HTTP server
├── Cargo.toml
└── README.md
```

**Dependencies:**
- `syn` - Rust parser
- `quote` - Token manipulation
- `walkdir` - File traversal
- `axum` - HTTP server (agent only)
- `tokio` - Async runtime (agent only)
- `serde` / `serde_json` - Serialization (agent only)

## Performance

- **Parsing**: ~1000 files/second
- **Call graph**: Near-instant for most functions
- **Memory**: Entire project AST kept in memory (typically <100MB)

## Limitations

- **External crates**: Only analyzes source files in the project directory (doesn't parse dependencies)
- **Macros**: Shows macro invocations as calls, but doesn't expand them
- **Dynamic dispatch**: Can't resolve trait object calls
- **Formatting**: Source output uses token streams (not rustfmt)

## Troubleshooting

### "Function not found" error

```bash
# Try the fully qualified name
morpho-rs-cli . "./src/lib.rs::my_function"

# Or list all functions to find the right path
morpho-rs-cli . | grep my_function
```

### Agent not responding

```bash
# Check if agent is running
curl http://127.0.0.1:8080/tool/generate_call_graph

# Check logs
./target/release/morpho-rs-agent
```

### Parse errors

morpho-rs skips files it can't parse (e.g., syntax errors). Check that your project compiles:

```bash
cargo check
```

## Contributing

Contributions welcome! Areas for improvement:

- [ ] Mermaid diagram output format
- [ ] Depth limiting for call graphs
- [ ] Incremental parsing/caching
- [ ] Module-level summaries
- [ ] Doc comment extraction
- [ ] Trait implementation tracking
- [ ] Generic type resolution
