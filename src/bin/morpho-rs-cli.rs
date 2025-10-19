// cli/main.rs

use morpho_rs::{generate_output_with_blacklist, OutputMode, VisibilityFilter};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <directory> [function] [--source] [--public-only] [--blacklist <paths>]",
            args[0]
        );
        eprintln!("  <directory>           - Directory to analyze");
        eprintln!("  [function]            - Optional: Function name for call graph or source view");
        eprintln!("  --source              - Show source code of function (requires function name)");
        eprintln!("  --public-only         - Show only public items");
        eprintln!("  --blacklist <paths>   - Comma-separated list of directories/paths to exclude (e.g., 'target,tests')");
        std::process::exit(1);
    }

    let dir = &args[1];
    if !std::path::Path::new(dir).is_dir() {
        eprintln!("Error: {} is not a directory", dir);
        std::process::exit(1);
    }

    // Check for flags
    let has_source = args.contains(&"--source".to_string());
    let has_public_only = args.contains(&"--public-only".to_string());

    // Parse blacklist
    let blacklist: Vec<String> = if let Some(pos) = args.iter().position(|arg| arg == "--blacklist") {
        if pos + 1 < args.len() {
            args[pos + 1]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            eprintln!("Error: --blacklist requires a comma-separated list of paths");
            std::process::exit(1);
        }
    } else {
        vec![]
    };

    let visibility = if has_public_only {
        VisibilityFilter::PublicOnly
    } else {
        VisibilityFilter::All
    };

    // Determine mode based on arguments
    // Check if args[2] exists and is not a flag
    let function_name = if args.len() > 2 && !args[2].starts_with("--") {
        Some(&args[2])
    } else {
        None
    };

    let mode = if let Some(func) = function_name {
        if has_source {
            // Show source code
            OutputMode::Source {
                function: func.to_string(),
            }
        } else {
            // Show call graph
            OutputMode::CallGraph {
                root: func.to_string(),
                visibility,
            }
        }
    } else {
        // Just directory (no function specified)
        OutputMode::ListAll { visibility }
    };

    match generate_output_with_blacklist(dir, mode, &blacklist) {
        Ok(output) => println!("{}", output.content),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
