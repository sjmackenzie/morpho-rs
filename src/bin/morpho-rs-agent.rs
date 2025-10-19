// agent/main.rs

use axum::{http::StatusCode, response::Json, routing::{get, post}, Router};
use morpho_rs::{generate_output_multi_dir, OutputMode, VisibilityFilter};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct ProjectInfo {
    full_path: String,
    short_name: String,
    is_primary: bool,
}

static PROJECT_DIRS: OnceLock<Vec<String>> = OnceLock::new();
static PROJECT_INFO: OnceLock<Vec<ProjectInfo>> = OnceLock::new();
static NAME_TO_PATH: OnceLock<HashMap<String, String>> = OnceLock::new();

#[derive(Deserialize)]
pub struct CallGraphRequest {
    root_function: String,
    public_only: Option<bool>,
    blacklist: Option<Vec<String>>,
    directory: Option<String>, // Filter to specific directory
}

#[derive(Deserialize)]
pub struct SourceRequest {
    function: String,
    blacklist: Option<Vec<String>>,
    directory: Option<String>, // Filter to specific directory
}

#[derive(Deserialize)]
pub struct ListAllRequest {
    public_only: Option<bool>,
    blacklist: Option<Vec<String>>,
    directory: Option<String>, // Filter to specific directory
}

#[derive(Serialize)]
pub struct ToolCallResponse {
    pub result: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize)]
pub struct ProjectInfoResponse {
    pub name: String,
    pub path: String,
}

#[derive(Serialize)]
pub struct InfoResponse {
    pub primary_project: ProjectInfoResponse,
    pub dependencies: Vec<ProjectInfoResponse>,
}

// Helper function to resolve directory name to full path
fn resolve_directory(name: &str) -> Result<String, String> {
    let name_map = NAME_TO_PATH.get().unwrap();

    // Check if it's a short name for a top-level project
    if let Some(path) = name_map.get(name) {
        return Ok(path.clone());
    }

    // Check if it starts with a short name followed by a path (e.g., "gpui-component/crates/ui")
    for (short_name, base_path) in name_map.iter() {
        if name.starts_with(&format!("{}/", short_name)) {
            // Extract the subpath after the short name
            let subpath = &name[short_name.len() + 1..];
            let full_path = format!("{}/{}", base_path, subpath);

            // Verify the directory exists
            if std::path::Path::new(&full_path).exists() {
                return Ok(full_path);
            } else {
                return Err(format!("Directory '{}' does not exist", full_path));
            }
        }
    }

    // Otherwise assume it's a full path
    let all_dirs = PROJECT_DIRS.get().unwrap();
    if all_dirs.contains(&name.to_string()) {
        return Ok(name.to_string());
    }

    // If it's a full path that exists, use it
    if std::path::Path::new(name).exists() {
        return Ok(name.to_string());
    }

    // Build helpful error message with available options
    let project_info = PROJECT_INFO.get().unwrap();
    let mut available = Vec::new();
    for info in project_info {
        available.push(format!("  '{}' -> {}", info.short_name, info.full_path));
    }
    available.push("\nYou can also use subdirectories: 'project-name/subdir/path'".to_string());

    Err(format!(
        "Unknown directory: '{}'. Available projects:\n{}",
        name,
        available.join("\n")
    ))
}

async fn get_info() -> Json<InfoResponse> {
    let project_info = PROJECT_INFO.get().unwrap();

    let primary = project_info.iter().find(|p| p.is_primary).unwrap();
    let dependencies: Vec<ProjectInfoResponse> = project_info
        .iter()
        .filter(|p| !p.is_primary)
        .map(|p| ProjectInfoResponse {
            name: p.short_name.clone(),
            path: p.full_path.clone(),
        })
        .collect();

    Json(InfoResponse {
        primary_project: ProjectInfoResponse {
            name: primary.short_name.clone(),
            path: primary.full_path.clone(),
        },
        dependencies,
    })
}

async fn generate_call_graph(
    Json(req): Json<CallGraphRequest>,
) -> Result<Json<ToolCallResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visibility = if req.public_only.unwrap_or(false) {
        VisibilityFilter::PublicOnly
    } else {
        VisibilityFilter::All
    };

    let blacklist = req.blacklist.unwrap_or_default();

    // Use specified directory or all directories
    let all_dirs = PROJECT_DIRS.get().unwrap();
    let dirs = if let Some(ref dir_name) = req.directory {
        match resolve_directory(dir_name) {
            Ok(resolved) => vec![resolved],
            Err(error_msg) => {
                return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: error_msg,
                })));
            }
        }
    } else {
        all_dirs.clone()
    };

    match generate_output_multi_dir(
        &dirs,
        OutputMode::CallGraph {
            root: req.root_function,
            visibility,
        },
        &blacklist,
    ) {
        Ok(output) => Ok(Json(ToolCallResponse {
            result: output.content,
        })),
        Err(e) => {
            eprintln!("Error generating call graph: {}", e);
            Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                error: e,
            })))
        }
    }
}

async fn get_source(
    Json(req): Json<SourceRequest>,
) -> Result<Json<ToolCallResponse>, (StatusCode, Json<ErrorResponse>)> {
    let blacklist = req.blacklist.unwrap_or_default();

    // Use specified directory or all directories
    let all_dirs = PROJECT_DIRS.get().unwrap();
    let dirs = if let Some(ref dir_name) = req.directory {
        match resolve_directory(dir_name) {
            Ok(resolved) => vec![resolved],
            Err(error_msg) => {
                return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: error_msg,
                })));
            }
        }
    } else {
        all_dirs.clone()
    };

    match generate_output_multi_dir(&dirs, OutputMode::Source { function: req.function }, &blacklist) {
        Ok(output) => Ok(Json(ToolCallResponse {
            result: output.content,
        })),
        Err(e) => {
            eprintln!("Error getting source: {}", e);
            Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                error: e,
            })))
        }
    }
}

async fn list_all(
    Json(req): Json<ListAllRequest>,
) -> Result<Json<ToolCallResponse>, (StatusCode, Json<ErrorResponse>)> {
    let visibility = if req.public_only.unwrap_or(false) {
        VisibilityFilter::PublicOnly
    } else {
        VisibilityFilter::All
    };

    let blacklist = req.blacklist.unwrap_or_default();

    // Use specified directory or all directories
    let all_dirs = PROJECT_DIRS.get().unwrap();
    let dirs = if let Some(ref dir_name) = req.directory {
        match resolve_directory(dir_name) {
            Ok(resolved) => vec![resolved],
            Err(error_msg) => {
                return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: error_msg,
                })));
            }
        }
    } else {
        all_dirs.clone()
    };

    match generate_output_multi_dir(&dirs, OutputMode::ListAll { visibility }, &blacklist) {
        Ok(output) => Ok(Json(ToolCallResponse {
            result: output.content,
        })),
        Err(e) => {
            eprintln!("Error listing all: {}", e);
            Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                error: e,
            })))
        }
    }
}

#[tokio::main]
async fn main() {
    // Determine project directories:
    // 1. CLI args (everything after program name)
    // 2. MORPHO_PROJECT_DIRS environment variable (colon-separated)
    // 3. Current directory as fallback
    let args: Vec<String> = std::env::args().skip(1).collect();

    let dirs = if !args.is_empty() {
        args
    } else if let Ok(env_dirs) = std::env::var("MORPHO_PROJECT_DIRS") {
        env_dirs.split(':').map(|s| s.to_string()).collect()
    } else {
        vec![".".to_string()]
    };

    // Build project info structures
    let mut project_info_vec = Vec::new();
    let mut name_to_path_map = HashMap::new();

    for (idx, dir) in dirs.iter().enumerate() {
        // Extract short name from path (last component)
        let short_name = std::path::Path::new(dir)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let info = ProjectInfo {
            full_path: dir.clone(),
            short_name: short_name.clone(),
            is_primary: idx == 0, // First one is primary
        };

        name_to_path_map.insert(short_name, dir.clone());
        project_info_vec.push(info);
    }

    PROJECT_DIRS.set(dirs.clone()).expect("Failed to set PROJECT_DIRS");
    PROJECT_INFO.set(project_info_vec.clone()).expect("Failed to set PROJECT_INFO");
    NAME_TO_PATH.set(name_to_path_map).expect("Failed to set NAME_TO_PATH");

    let app = Router::new()
        .route("/info", get(get_info))
        .route("/tool/generate_call_graph", post(generate_call_graph))
        .route("/tool/get_source", post(get_source))
        .route("/tool/list_all", post(list_all));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    println!("🚀 morpho-rs-agent (HTTP) listening on http://127.0.0.1:8080");
    println!("   Primary project: {} ({})",
        project_info_vec[0].short_name,
        project_info_vec[0].full_path
    );

    if project_info_vec.len() > 1 {
        println!("   Dependencies:");
        for info in &project_info_vec[1..] {
            println!("     - {} ({})", info.short_name, info.full_path);
        }
    }

    println!("\n   Available endpoints:");
    println!("   GET  /info                    - Get project and dependency information");
    println!("   POST /tool/generate_call_graph - Generate call graph from a function");
    println!("   POST /tool/get_source          - Get source code of a function");
    println!("   POST /tool/list_all            - List all types and functions in project");

    axum::serve(listener, app).await.unwrap();
}
