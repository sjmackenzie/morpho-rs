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

**Exclude directories** (e.g., skip generated code, tests, benchmarks):

```bash
morpho-rs-cli /path/to/rust/project --blacklist target,tests,benches
```

**Combine filters**:

```bash
morpho-rs-cli /path/to/rust/project --public-only --blacklist target,examples
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
# Run the agent in the current directory (listens on http://127.0.0.1:8080)
morpho-rs-agent

# Specify a single project directory
morpho-rs-agent /path/to/your/rust/project

# Analyze multiple directories (e.g., main project + dependencies)
morpho-rs-agent /path/to/main/project /path/to/dependency1 /path/to/dependency2

# Or use environment variable (colon-separated paths)
export MORPHO_PROJECT_DIRS="/path/to/project:/path/to/dep1:/path/to/dep2"
morpho-rs-agent
```

**Output:**
```
🚀 morpho-rs-agent (HTTP) listening on http://127.0.0.1:8080
   Project directories: /path/to/project, /path/to/dep1
   /tool/generate_call_graph - Generate call graph from a function
   /tool/get_source          - Get source code of a function
   /tool/list_all            - List all types and functions in project
```

**Multi-Directory Support:**

The agent can analyze multiple Rust projects simultaneously, which is useful for:
- Including local dependencies in analysis
- Tracing function calls across crate boundaries
- Analyzing workspace members together

Priority order for directory configuration:
1. **Command-line arguments** - All arguments after the program name
2. **Environment variable** - `MORPHO_PROJECT_DIRS` (colon-separated)
3. **Current directory** - Falls back to `.` if nothing is specified

### API Endpoints

#### 0. Get Project Information

**Endpoint:** `GET /info`

**Response:**
```json
{
  "primary_project": {
    "name": "sio",
    "path": "/Users/rivergod/dev/sio"
  },
  "dependencies": [
    {
      "name": "gpui-component",
      "path": "/Users/rivergod/dev/gpui-component"
    },
    {
      "name": "werbolg",
      "path": "/Users/rivergod/dev/werbolg"
    }
  ]
}
```

**Description:**
Returns information about the primary project and its dependencies. This endpoint:
- Identifies which directory is the primary project (the first one passed to `morpho-rs-agent`)
- Lists all dependency directories
- Provides short names that can be used in the `directory` parameter of other endpoints

**cURL Example:**
```bash
curl http://127.0.0.1:8080/info
```

#### 1. List All Items

**Endpoint:** `POST /tool/list_all`

**Request Body:**
```json
{
  "public_only": false,
  "blacklist": ["target", "tests", "benches"],
  "directory": "/path/to/specific/codebase"
}
```

**Parameters:**
- `public_only` (optional, boolean): Only show public items
- `blacklist` (optional, array of strings): Directories/paths to exclude
- `directory` (optional, string): Filter to specific project or subdirectory. Examples:
  - `"gpui-component"` - entire project
  - `"gpui-component/crates/ui/src/button"` - specific subdirectory
  - `"sio/src"` - subdirectory within sio project
  - Use `GET /info` to see top-level projects. If not specified, searches all configured directories

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
  "public_only": false,
  "blacklist": ["target", "tests"],
  "directory": "/path/to/specific/codebase"
}
```

**Parameters:**
- `root_function` (required, string): Function to analyze
- `public_only` (optional, boolean): Only show public functions
- `blacklist` (optional, array of strings): Directories/paths to exclude
- `directory` (optional, string): Filter to specific project or subdirectory. Examples:
  - `"gpui-component"` - entire project
  - `"gpui-component/crates/ui"` - specific subdirectory
  - Use `GET /info` to see top-level projects. If not specified, searches all configured directories

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

#### 3. Get Function or Type Source

**Endpoint:** `POST /tool/get_source`

**Request Body:**
```json
{
  "function": "./src/lib.rs::generate_output",
  "blacklist": ["target"],
  "directory": "/path/to/specific/codebase"
}
```

**Parameters:**
- `function` (required, string): Function or type name to retrieve source for (e.g., `"Button"`, `"main"`, `"./src/lib.rs::Button"`)
- `blacklist` (optional, array of strings): Directories/paths to exclude
- `directory` (optional, string): Filter to specific project or subdirectory. Examples:
  - `"gpui-component"` - entire project
  - `"gpui-component/crates/ui"` - specific subdirectory
  - Use `GET /info` to see top-level projects. If not specified, searches all configured directories

**Note:** This endpoint works for both functions and types (structs, enums, etc.). It will search for functions first, then types if no function is found.

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

The `morpho-rs-agent` HTTP server can be integrated with various AI coding assistants. Pre-built integration scripts are available in the `integration/` directory.

### Quick Start

1. **Start the agent** in your Rust project directory:
```bash
cd /path/to/your/rust/project
morpho-rs-agent

# Or specify multiple directories to include dependencies:
morpho-rs-agent /path/to/main/project /path/to/local/dependency
```

2. **Choose your AI tool** and follow the setup instructions below

### Supported Integrations

#### Claude Code (MCP)
Uses Model Context Protocol for seamless integration.

**Setup:**
```bash
cd integration/javascript
chmod +x claude_mcp_server.js

# Add to .claude/config.json:
# {
#   "mcpServers": {
#     "morpho-rs": {
#       "command": "/absolute/path/to/integration/javascript/claude_mcp_server.js"
#     }
#   }
# }
```

See [`integration/javascript/README.md`](integration/javascript/README.md) for details.

#### LM Studio 3+
Uses MCP (Model Context Protocol).

**Setup:**
```bash
cd integration/javascript
chmod +x lm_studio_mcp_server.js

# Add to LM Studio MCP settings:
# {
#   "mcpServers": {
#     "morpho-rs": {
#       "command": "/absolute/path/to/integration/javascript/lm_studio_mcp_server.js"
#     }
#   }
# }
```

See [`integration/javascript/README.md`](integration/javascript/README.md) for details.

**Note:** For older LM Studio versions (pre-v3), use the Python function calling integration in [`integration/python/README.md`](integration/python/README.md).

#### Qwen Coder
Direct tool integration via OpenAI-compatible API.

**Setup:**
```bash
cd integration/python
pip install openai requests
python qwen_tools.py
```

See [`integration/python/README.md`](integration/python/README.md) for details.

### Custom Integration (Rust)

For integrating into your own Rust application:

```rust
use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    // List all public items
    let res = client
        .post("http://127.0.0.1:8080/tool/list_all")
        .json(&json!({
            "public_only": true,
            "blacklist": ["target", "tests"]
        }))
        .send()
        .await?;

    println!("{}", res.json::<serde_json::Value>().await?["result"]);
    Ok(())
}
```

**Dependencies:**
```toml
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
```

### Other AI Tools

Most AI coding assistants can call HTTP endpoints directly. Point them to:
- `POST http://127.0.0.1:8080/tool/list_all`
- `POST http://127.0.0.1:8080/tool/generate_call_graph`
- `POST http://127.0.0.1:8080/tool/get_source`

See the [API Endpoints](#api-endpoints) section for request/response formats.

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

### 6. Multi-Project Analysis (Agent Only)

The HTTP agent supports analyzing multiple directories simultaneously:

```bash
# Start agent with main project + local dependencies
morpho-rs-agent /path/to/main/project /path/to/local/dep1 /path/to/local/dep2

# Now you can trace calls across crate boundaries
# For example, see how your main project calls into dependency code
curl -X POST http://127.0.0.1:8080/tool/generate_call_graph \
  -H "Content-Type: application/json" \
  -d '{"root_function": "./main/src/lib.rs::process_data"}'

# The call graph will show calls into functions from all three directories

# Check which projects are available
curl http://127.0.0.1:8080/info

# Filter to just one codebase using short name
curl -X POST http://127.0.0.1:8080/tool/list_all \
  -H "Content-Type: application/json" \
  -d '{"public_only": true, "directory": "sio"}'

# Filter to a specific subdirectory
curl -X POST http://127.0.0.1:8080/tool/list_all \
  -H "Content-Type: application/json" \
  -d '{"public_only": true, "directory": "gpui-component/crates/ui/src/button"}'

# This will only show items from the button subdirectory
```

**Directory Filtering with Short Names and Subdirectories:**

The agent automatically extracts short names from directory paths (e.g., `/Users/rivergod/dev/sio` → `sio`). You can:
1. Call `GET /info` to see the primary project and all dependencies with their short names
2. Use short names for entire projects: `"directory": "gpui-component"`
3. Use subdirectory paths: `"directory": "gpui-component/crates/ui/src/button"`
4. Or use full paths if preferred: `"directory": "/Users/rivergod/dev/gpui-component"`

Benefits:
- **Without `directory`** - Returns results from all configured directories (may be noisy)
- **With project name** - Returns results only from that project (e.g., `"gpui-component"`)
- **With subdirectory** - Returns results only from that subdirectory (e.g., `"gpui-component/crates/ui"`)
- **Short names** - More readable and portable than full paths

This is especially useful for:
- **Workspace analysis** - Analyze all workspace members together
- **Local dependencies** - Include git submodules or path dependencies
- **Cross-crate refactoring** - Understand impact across multiple crates
- **Focused queries** - Filter to specific codebase to reduce token usage

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

### Blacklist Filtering

Exclude specific directories or paths from analysis to reduce noise and improve performance:

```bash
# Exclude common directories
morpho-rs-cli . --blacklist target,tests,benches,examples

# Exclude generated code
morpho-rs-cli . --blacklist target,build,generated

# Works with all modes
morpho-rs-cli . "main" --blacklist target
morpho-rs-cli . "main" --source --blacklist tests,benches
```

**Use Cases:**
- **Skip build artifacts**: `target` directory
- **Ignore test code**: `tests`, `benches`
- **Exclude examples**: `examples`
- **Skip generated code**: `build`, `generated`, `proto`
- **Ignore vendor code**: `vendor`, `third_party`

**How it works:**
- Blacklist uses substring matching on file paths
- Applies to both directories and individual files
- Multiple paths separated by commas
- Case-sensitive matching

**Example:**
```bash
# Before (with tests included)
morpho-rs-cli . | wc -l
1500 lines

# After (tests excluded)
morpho-rs-cli . --blacklist tests | wc -l
800 lines  # 47% reduction!
```

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
