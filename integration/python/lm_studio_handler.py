#!/usr/bin/env python3
"""
LM Studio function handler for morpho-rs

This script handles function calls from LM Studio and forwards them to morpho-rs agent.

Setup:
    1. Start morpho-rs-agent in your Rust project directory
    2. In LM Studio → Settings → Functions:
       - Enable "Function Calling"
       - Load morpho_functions.json
       - Set handler script to this file

Usage:
    LM Studio will call this script via stdin/stdout
"""

import json
import requests
import sys

def handle_function_call(function_name, arguments):
    """Handle function call from LM Studio"""
    base_url = "http://127.0.0.1:8080"

    if function_name == "list_rust_items":
        response = requests.post(
            f"{base_url}/tool/list_all",
            json={
                "public_only": arguments.get("public_only", False),
                "blacklist": arguments.get("blacklist", [])
            }
        )
    elif function_name == "analyze_rust_callgraph":
        response = requests.post(
            f"{base_url}/tool/generate_call_graph",
            json={
                "root_function": arguments["function"],
                "public_only": arguments.get("public_only", False),
                "blacklist": arguments.get("blacklist", [])
            }
        )
    elif function_name == "get_rust_source":
        response = requests.post(
            f"{base_url}/tool/get_source",
            json={
                "function": arguments["function"],
                "blacklist": arguments.get("blacklist", [])
            }
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
