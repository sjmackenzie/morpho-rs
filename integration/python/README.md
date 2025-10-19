# Python Integration for morpho-rs

This directory contains Python scripts for integrating morpho-rs with various AI coding tools.

## Files

### `qwen_tools.py`
Tool definitions and wrapper for Qwen Coder.

**Usage:**
```bash
# 1. Start morpho-rs-agent in your Rust project
cd /path/to/rust/project
morpho-rs-agent

# Or include multiple directories (e.g., with dependencies):
morpho-rs-agent /path/to/main/project /path/to/dependency

# 2. Run the Qwen integration
python qwen_tools.py
```

**Requirements:**
```bash
pip install openai requests
```

### `lm_studio_handler.py`
Function handler for LM Studio.

**Setup:**
1. Start `morpho-rs-agent` in your Rust project directory
2. In LM Studio → Settings → Functions:
   - Enable "Function Calling"
   - Load `morpho_functions.json`
   - Set handler script to `lm_studio_handler.py`

**Requirements:**
```bash
pip install requests
```

### `morpho_functions.json`
Function definitions for LM Studio.

Load this file in LM Studio's function calling configuration.

## Common Requirements

All Python scripts require:
- Python 3.7+
- `requests` library
- morpho-rs-agent running on `http://127.0.0.1:8080`

## Examples

### Using with Qwen Coder

```python
from qwen_tools import call_tool

# List all public items
result = call_tool("list_rust_items", {"public_only": True})
print(result)

# Analyze a function
result = call_tool("analyze_rust_callgraph", {
    "function": "./src/lib.rs::main",
    "public_only": False,
    "blacklist": ["target", "tests"]
})
print(result)
```

### Direct HTTP Calls

```python
import requests

# Call morpho-rs agent directly
response = requests.post(
    "http://127.0.0.1:8080/tool/list_all",
    json={"public_only": True, "blacklist": ["target"]}
)

print(response.json()["result"])
```
