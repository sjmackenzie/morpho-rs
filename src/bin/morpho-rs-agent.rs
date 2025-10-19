// agent/main.rs

use axum::{http::StatusCode, response::Json, routing::post, Router};
use morpho_rs::{generate_output, OutputMode, VisibilityFilter};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CallGraphRequest {
    root_function: String,
    public_only: Option<bool>,
}

#[derive(Deserialize)]
pub struct SourceRequest {
    function: String,
}

#[derive(Deserialize)]
pub struct ListAllRequest {
    public_only: Option<bool>,
}

#[derive(Serialize)]
pub struct ToolCallResponse {
    pub result: String,
}

async fn generate_call_graph(
    Json(req): Json<CallGraphRequest>,
) -> Result<Json<ToolCallResponse>, StatusCode> {
    let visibility = if req.public_only.unwrap_or(false) {
        VisibilityFilter::PublicOnly
    } else {
        VisibilityFilter::All
    };

    match generate_output(
        ".",
        OutputMode::CallGraph {
            root: req.root_function,
            visibility,
        },
    ) {
        Ok(output) => Ok(Json(ToolCallResponse {
            result: output.content,
        })),
        Err(e) => {
            eprintln!("Error generating call graph: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn get_source(
    Json(req): Json<SourceRequest>,
) -> Result<Json<ToolCallResponse>, StatusCode> {
    match generate_output(".", OutputMode::Source { function: req.function }) {
        Ok(output) => Ok(Json(ToolCallResponse {
            result: output.content,
        })),
        Err(e) => {
            eprintln!("Error getting source: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

async fn list_all(
    Json(req): Json<ListAllRequest>,
) -> Result<Json<ToolCallResponse>, StatusCode> {
    let visibility = if req.public_only.unwrap_or(false) {
        VisibilityFilter::PublicOnly
    } else {
        VisibilityFilter::All
    };

    match generate_output(".", OutputMode::ListAll { visibility }) {
        Ok(output) => Ok(Json(ToolCallResponse {
            result: output.content,
        })),
        Err(e) => {
            eprintln!("Error listing all: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/tool/generate_call_graph", post(generate_call_graph))
        .route("/tool/get_source", post(get_source))
        .route("/tool/list_all", post(list_all));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    println!("🚀 morpho-rs-agent (HTTP) listening on http://127.0.0.1:8080");
    println!("   /tool/generate_call_graph - Generate call graph from a function");
    println!("   /tool/get_source          - Get source code of a function");
    println!("   /tool/list_all            - List all types and functions in project");
    axum::serve(listener, app).await.unwrap();
}
