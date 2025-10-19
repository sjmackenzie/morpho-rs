// cli/main.rs

use morpho_rs::{generate_output, OutputMode, VisibilityFilter};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <directory> [function] [--source] [--public-only]",
            args[0]
        );
        eprintln!("  <directory>           - Directory to analyze");
        eprintln!("  [function]            - Optional: Function name for call graph or source view");
        eprintln!("  --source              - Show source code of function (requires function name)");
        eprintln!("  --public-only         - Show only public items");
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

    let visibility = if has_public_only {
        VisibilityFilter::PublicOnly
    } else {
        VisibilityFilter::All
    };

    // Determine mode based on arguments
    let mode = if args.len() == 2 || (args.len() == 3 && has_public_only) {
        // Just directory (and maybe --public-only)
        OutputMode::ListAll { visibility }
    } else {
        // Has a function name
        let function_name = &args[2];

        if has_source {
            // Show source code
            OutputMode::Source {
                function: function_name.to_string(),
            }
        } else {
            // Show call graph
            OutputMode::CallGraph {
                root: function_name.to_string(),
                visibility,
            }
        }
    };

    match generate_output(dir, mode) {
        Ok(output) => println!("{}", output.content),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
