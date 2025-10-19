#!/usr/bin/env python3
"""
Qwen Coder integration for morpho-rs

This script provides tool definitions for Qwen Coder to analyze Rust code
using the morpho-rs agent.

Usage:
    1. Start morpho-rs-agent in your Rust project directory
    2. Run: python qwen_tools.py
"""

import requests
from openai import OpenAI

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
                    },
                    "blacklist": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Directories to exclude (e.g., ['target', 'tests'])",
                        "default": []
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
                    },
                    "blacklist": {
                        "type": "array",
                        "items": {"type": "string"},
                        "default": []
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
                    },
                    "blacklist": {
                        "type": "array",
                        "items": {"type": "string"},
                        "default": []
                    }
                },
                "required": ["function"]
            }
        }
    }
]

def call_tool(tool_name, arguments):
    """Call morpho-rs agent and return result"""
    if tool_name == "list_rust_items":
        resp = requests.post(
            f"{MORPHO_BASE}/tool/list_all",
            json={
                "public_only": arguments.get("public_only", False),
                "blacklist": arguments.get("blacklist", [])
            }
        )
        return resp.json()["result"]
    elif tool_name == "analyze_rust_callgraph":
        resp = requests.post(
            f"{MORPHO_BASE}/tool/generate_call_graph",
            json={
                "root_function": arguments["function"],
                "public_only": arguments.get("public_only", False),
                "blacklist": arguments.get("blacklist", [])
            }
        )
        return resp.json()["result"]
    elif tool_name == "get_rust_source":
        resp = requests.post(
            f"{MORPHO_BASE}/tool/get_source",
            json={
                "function": arguments["function"],
                "blacklist": arguments.get("blacklist", [])
            }
        )
        return resp.json()["result"]

# Example usage
if __name__ == "__main__":
    client = OpenAI(
        base_url="http://localhost:1234/v1",  # Adjust to your Qwen server
        api_key="not-needed"
    )

    messages = [
        {"role": "user", "content": "List all public items in the current Rust project"}
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
    else:
        print(response.choices[0].message.content)
