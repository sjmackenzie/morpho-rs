#!/usr/bin/env node

/**
 * MCP Server for LM Studio 3
 *
 * This server allows LM Studio 3 to analyze Rust projects using morpho-rs.
 *
 * Setup:
 *   1. Make this file executable: chmod +x lm_studio_mcp_server.js
 *   2. Add to LM Studio MCP settings:
 *      {
 *        "mcpServers": {
 *          "morpho-rs": {
 *            "command": "/absolute/path/to/integration/javascript/lm_studio_mcp_server.js"
 *          }
 *        }
 *      }
 *   3. Start morpho-rs-agent in your Rust project directory
 *
 * Usage:
 *   Ask LM Studio to analyze your Rust code and it will use morpho-rs automatically
 */

const http = require("http");

// Tool definitions
const TOOLS = [
  {
    name: "get_project_info",
    description: "Get information about the primary project and dependencies. Returns the project names and paths available for analysis.",
    inputSchema: {
      type: "object",
      properties: {},
    },
  },
  {
    name: "list_rust_items",
    description: "List all types and functions in a Rust project. Call get_project_info first to see available codebases.",
    inputSchema: {
      type: "object",
      properties: {
        public_only: {
          type: "boolean",
          description: "Only show public API items (reduces token usage)",
          default: false,
        },
        blacklist: {
          type: "array",
          items: { type: "string" },
          description: 'Directories to exclude (e.g., ["target", "tests"])',
          default: [],
        },
        directory: {
          type: "string",
          description: "Optional: Filter to a specific project or subdirectory. Examples: 'gpui-component' (entire project), 'gpui-component/crates/ui/src/button' (specific subdirectory), 'sio/src' (subdirectory). Call get_project_info to see top-level projects. If omitted, searches all projects.",
        },
      },
    },
  },
  {
    name: "analyze_rust_callgraph",
    description: "Show hierarchical call graph for a Rust function. Call get_project_info first to see available codebases.",
    inputSchema: {
      type: "object",
      properties: {
        function: {
          type: "string",
          description:
            'Function to analyze (e.g., "./src/lib.rs::main" or "main")',
        },
        public_only: {
          type: "boolean",
          description: "Only show public functions",
          default: false,
        },
        blacklist: {
          type: "array",
          items: { type: "string" },
          default: [],
        },
        directory: {
          type: "string",
          description: "Optional: Filter to a specific project or subdirectory. Examples: 'gpui-component' (entire project), 'gpui-component/crates/ui/src/button' (specific subdirectory), 'sio/src' (subdirectory). Call get_project_info to see top-level projects. If omitted, searches all projects.",
        },
      },
      required: ["function"],
    },
  },
  {
    name: "get_rust_source",
    description: "Get formatted source code of a Rust function or type (struct/enum). Call get_project_info first to see available codebases.",
    inputSchema: {
      type: "object",
      properties: {
        function: {
          type: "string",
          description: "Function or type name to retrieve source for (e.g., 'Button', 'main', './src/lib.rs::Button')",
        },
        blacklist: {
          type: "array",
          items: { type: "string" },
          default: [],
        },
        directory: {
          type: "string",
          description: "Optional: Filter to a specific project or subdirectory. Examples: 'gpui-component' (entire project), 'gpui-component/crates/ui/src/button' (specific subdirectory), 'sio/src' (subdirectory). Call get_project_info to see top-level projects. If omitted, searches all projects.",
        },
      },
      required: ["function"],
    },
  },
];

// Call morpho-rs agent (GET request)
function callMorphoGet(endpoint) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: "127.0.0.1",
      port: 8080,
      path: endpoint,
      method: "GET",
    };

    const req = http.request(options, (res) => {
      let body = "";
      res.on("data", (chunk) => (body += chunk));
      res.on("end", () => {
        try {
          const parsed = JSON.parse(body);
          if (parsed.error) {
            reject(new Error(parsed.error));
          } else {
            resolve(parsed);
          }
        } catch (e) {
          reject(new Error(`Failed to parse response: ${e.message}. Body: ${body}`));
        }
      });
    });

    req.on("error", reject);
    req.end();
  });
}

// Call morpho-rs agent (POST request)
function callMorpho(endpoint, data) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: "127.0.0.1",
      port: 8080,
      path: endpoint,
      method: "POST",
      headers: { "Content-Type": "application/json" },
    };

    const req = http.request(options, (res) => {
      let body = "";
      res.on("data", (chunk) => (body += chunk));
      res.on("end", () => {
        try {
          const parsed = JSON.parse(body);
          if (parsed.error) {
            reject(new Error(parsed.error));
          } else {
            resolve(parsed);
          }
        } catch (e) {
          reject(new Error(`Failed to parse response: ${e.message}. Body: ${body}`));
        }
      });
    });

    req.on("error", reject);
    req.write(JSON.stringify(data));
    req.end();
  });
}

// Handle MCP requests
async function handleRequest(request) {
  const { method, params } = request;

  switch (method) {
    case "initialize":
      return {
        protocolVersion: "2024-11-05",
        capabilities: {
          tools: {},
        },
        serverInfo: {
          name: "morpho-rs",
          version: "0.1.0",
        },
      };

    case "tools/list":
      return { tools: TOOLS };

    case "tools/call": {
      const { name, arguments: args } = params;

      // Handle get_project_info separately (GET request)
      if (name === "get_project_info") {
        const response = await callMorphoGet("/info");

        // Format the response nicely
        let text = `Primary Project: ${response.primary_project.name} (${response.primary_project.path})\n`;
        if (response.dependencies.length > 0) {
          text += "\nDependencies:\n";
          for (const dep of response.dependencies) {
            text += `  - ${dep.name} (${dep.path})\n`;
          }
        }

        return {
          content: [
            {
              type: "text",
              text: text,
            },
          ],
        };
      }

      let endpoint, data;

      if (name === "list_rust_items") {
        endpoint = "/tool/list_all";
        data = {
          public_only: args.public_only || false,
          blacklist: args.blacklist || [],
        };
        if (args.directory) {
          data.directory = args.directory;
        }
      } else if (name === "analyze_rust_callgraph") {
        endpoint = "/tool/generate_call_graph";
        data = {
          root_function: args.function,
          public_only: args.public_only || false,
          blacklist: args.blacklist || [],
        };
        if (args.directory) {
          data.directory = args.directory;
        }
      } else if (name === "get_rust_source") {
        endpoint = "/tool/get_source";
        data = {
          function: args.function,
          blacklist: args.blacklist || [],
        };
        if (args.directory) {
          data.directory = args.directory;
        }
      } else {
        throw new Error(`Unknown tool: ${name}`);
      }

      const response = await callMorpho(endpoint, data);

      return {
        content: [
          {
            type: "text",
            text: response.result,
          },
        ],
      };
    }

    default:
      throw new Error(`Unknown method: ${method}`);
  }
}

// MCP stdio protocol
const readline = require("readline");
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false,
});

rl.on("line", async (line) => {
  let request;
  try {
    request = JSON.parse(line);
    const result = await handleRequest(request);

    const response = {
      jsonrpc: "2.0",
      id: request.id,
      result,
    };

    console.log(JSON.stringify(response));
  } catch (error) {
    const response = {
      jsonrpc: "2.0",
      id: request?.id || null,
      error: {
        code: -32603,
        message: error.message,
      },
    };

    console.log(JSON.stringify(response));
  }
});

process.on("SIGINT", () => process.exit(0));
process.on("SIGTERM", () => process.exit(0));
