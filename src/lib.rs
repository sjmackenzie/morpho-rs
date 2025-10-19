use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use syn::{Block, Expr, FnArg, Item, Type, Visibility};
use walkdir::WalkDir;

// ============= PUBLIC API TYPES =============
#[derive(Clone)]
pub struct Function {
    pub vis: Visibility,
    pub sig: syn::Signature,
    pub block: Option<Block>,
    pub qualified_name: String, // e.g., "main" or "MyStruct::new"
}

#[derive(Debug, Clone)]
pub struct CallSite {
    pub name: String,
    pub context: Option<String>, // e.g., "if (x > 0)", "match Some(_)"
}

#[derive(Clone)]
pub struct Project {
    pub functions: HashMap<String, Function>, // keyed by qualified_name
    pub types: HashMap<String, (String, Item)>, // key = type name; value = (file_path, item)
}

#[derive(Debug, Clone, Copy)]
pub enum VisibilityFilter {
    All,
    PublicOnly,
}

#[derive(Debug)]
pub enum OutputMode {
    ListAll { visibility: VisibilityFilter },
    CallGraph { root: String, visibility: VisibilityFilter },
    Source { function: String },
}

#[derive(Debug)]
pub struct Output {
    pub content: String,
}

// ============= CORE LOGIC (NO I/O) =============
pub fn load_project(dir: &str) -> Result<Project, String> {
    let mut project = Project {
        functions: HashMap::new(),
        types: HashMap::new(),
    };

    for entry in WalkDir::new(dir).follow_links(true) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() || entry.path().extension().map_or(false, |e| e != "rs") {
            continue;
        }

        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let file = match syn::parse_file(&content) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let file_path_str = entry.path().to_string_lossy().into_owned();

        for item in file.items {
            match &item {
                syn::Item::Fn(f) => {
                    let fn_item = Function::from_fn(&f, &file_path_str);
                    project
                        .functions
                        .insert(fn_item.qualified_name.clone(), fn_item);
                }
                syn::Item::Impl(imp) => {
                    let impl_target_str = format_type(&imp.self_ty);
                    for item in &imp.items {
                        if let syn::ImplItem::Fn(method) = item {
                            let vis = method.vis.clone();
                            if matches!(&vis, syn::Visibility::Public(_)) {
                                let fn_item =
                                    Function::from_impl_method(method, impl_target_str.clone(), &file_path_str);
                                project
                                    .functions
                                    .insert(fn_item.qualified_name.clone(), fn_item);
                            }
                        }
                    }
                }

                syn::Item::Struct(s) => {
                    project
                        .types
                        .insert(s.ident.to_string(), (file_path_str.clone(), item.clone()));
                }
                syn::Item::Enum(e) => {
                    project
                        .types
                        .insert(e.ident.to_string(), (file_path_str.clone(), item.clone()));
                }
                syn::Item::Trait(t) => {
                    project
                        .types
                        .insert(t.ident.to_string(), (file_path_str.clone(), item.clone()));
                }
                syn::Item::Type(t) => {
                    project
                        .types
                        .insert(t.ident.to_string(), (file_path_str.clone(), item.clone()));
                }
                _ => {}
            }
        }
    }

    Ok(project)
}

impl Function {
    pub fn signature(&self) -> String {
        let vis = visibility_to_string(&self.vis);
        let asyncness = if self.sig.asyncness.is_some() {
            "async "
        } else {
            ""
        };
        let constness = if self.sig.constness.is_some() {
            "const "
        } else {
            ""
        };
        let unsafety = if self.sig.unsafety.is_some() {
            "unsafe "
        } else {
            ""
        };
        let args = format_args(&self.sig.inputs.iter().collect::<Vec<_>>());
        let ret = match &self.sig.output {
            syn::ReturnType::Default => "()".to_string(),
            syn::ReturnType::Type(_, ty) => format_type(ty),
        };

        format!(
            "{}{}{}{}fn {}({}) -> {}",
            vis, asyncness, constness, unsafety, self.qualified_name, args, ret
        )
    }

    pub fn full_body(&self) -> String {
        let sig = self.signature();
        if let Some(block) = &self.block {
            format!("{}\n{{\n{}}}\n", sig, indent_block(block))
        } else {
            format!("{}\n{{ ... }}\n", sig)
        }
    }

    pub fn calls(&self) -> Vec<CallSite> {
        let mut calls = vec![];
        if let Some(block) = &self.block {
            extract_calls_from_block(&block, &mut calls);
        }
        calls
    }

    pub fn from_fn(f: &syn::ItemFn, file_path: &str) -> Self {
        Function {
            vis: f.vis.clone(),
            sig: f.sig.clone(),
            block: Some(*f.block.clone()),
            qualified_name: format!("{}::{}", file_path, f.sig.ident),
        }
    }

    pub fn from_impl_method(method: &syn::ImplItemFn, impl_target_str: String, file_path: &str) -> Self {
        Function {
            vis: method.vis.clone(),
            sig: method.sig.clone(),
            block: Some(method.block.clone()),
            qualified_name: format!("{}::{}::{}", file_path, impl_target_str, method.sig.ident),
        }
    }
}

pub fn trace_calls(
    root_func: &str,
    project: &Project,
) -> Result<(HashSet<String>, HashSet<String>), String> {
    let mut visited = HashSet::new();
    let mut reachable_types = HashSet::<String>::new();

    if !project.functions.contains_key(root_func) {
        return Err(format!("Function '{}' not found", root_func));
    }

    _trace_calls(root_func, project, &mut visited, &mut reachable_types);

    Ok((visited, reachable_types))
}

fn _trace_calls(
    func_name: &str,
    project: &Project,
    visited: &mut HashSet<String>,
    reachable_types: &mut HashSet<String>,
) {
    // Try exact match first, then try to find by short name
    let func_entry = project.functions.get_key_value(func_name).or_else(|| {
        // If not found, try to find a function whose qualified name ends with ::func_name
        project.functions.iter()
            .find(|(qualified_name, _)| {
                qualified_name.ends_with(&format!("::{}", func_name))
            })
    });

    let (qualified_name, func) = match func_entry {
        Some((qn, f)) => (qn, f),
        None => {
            // Function not found - this can happen for external crate functions, macros, etc.
            // Just skip it silently
            return;
        }
    };

    // Use the actual qualified name for visited tracking
    if !visited.insert(qualified_name.clone()) {
        return;
    }

    collect_types_in_signature(&func.sig, reachable_types);

    for callee in &func.calls() {
        _trace_calls(&callee.name, project, visited, reachable_types);
    }
}

pub fn generate_output(dir: &str, mode: OutputMode) -> Result<Output, String> {
    let project = load_project(dir)?;

    match mode {
        OutputMode::ListAll { visibility } => generate_list_all(&project, visibility),
        OutputMode::CallGraph { root, visibility } => {
            let (visited_funcs, reachable_types) = trace_calls(&root, &project)?;

            // Filter functions and types by reachability
            let mut file_to_funcs: HashMap<String, Vec<Function>> = HashMap::new();
            for (name, func) in &project.functions {
                if visited_funcs.contains(name) {
                    let file = find_file_for_function(&func.qualified_name, &project)?;
                    file_to_funcs.entry(file).or_default().push(func.clone());
                }
            }

            let mut file_to_types: HashMap<String, Vec<Item>> = HashMap::new();
            for (type_name, (_, item)) in &project.types {
                if reachable_types.contains(type_name) {
                    let file = find_file_for_type(&type_name, &project)?;
                    file_to_types.entry(file).or_default().push(item.clone());
                }
            }

            generate_call_graph_output(&file_to_funcs, &file_to_types, visibility, Some(&root))
        }
        OutputMode::Source { function } => generate_source(&project, &function),
    }
}

// === INTERNAL HELPERS (no I/O) ===

fn generate_source(project: &Project, function_name: &str) -> Result<Output, String> {
    // Try to find the function
    let func = project.functions.get(function_name).or_else(|| {
        // Try suffix match
        project.functions.iter()
            .find(|(qn, _)| qn.ends_with(&format!("::{}", function_name)))
            .map(|(_, f)| f)
    });

    let func = match func {
        Some(f) => f,
        None => return Err(format!("Function '{}' not found", function_name)),
    };

    let mut output = String::new();

    // Get file path for header
    let file_path = find_file_for_function(&func.qualified_name, project)?;
    output.push_str(&format!("=== {} ===\n", file_path));

    // Output the function with properly formatted source
    output.push_str(&format_function_source(func));

    Ok(Output { content: output })
}

fn format_function_source(func: &Function) -> String {
    let vis = visibility_to_string(&func.vis);
    let asyncness = if func.sig.asyncness.is_some() { "async " } else { "" };
    let constness = if func.sig.constness.is_some() { "const " } else { "" };
    let unsafety = if func.sig.unsafety.is_some() { "unsafe " } else { "" };

    let args = format_args(&func.sig.inputs.iter().collect::<Vec<_>>());
    let ret = match &func.sig.output {
        syn::ReturnType::Default => "".to_string(),
        syn::ReturnType::Type(_, ty) => format!(" -> {}", format_type(ty)),
    };

    // Get just the function name without file path for display
    let display_name = if let Some(first_separator) = func.qualified_name.find("::") {
        &func.qualified_name[first_separator + 2..]
    } else {
        &func.qualified_name
    };

    if let Some(block) = &func.block {
        // Use the raw token stream for the block to preserve formatting
        let block_str = block.to_token_stream().to_string();
        format!(
            "{}{}{}{}fn {}({}){} {}\n",
            vis, asyncness, constness, unsafety, display_name, args, ret, block_str
        )
    } else {
        format!(
            "{}{}{}{}fn {}({}){} {{ ... }}\n",
            vis, asyncness, constness, unsafety, display_name, args, ret
        )
    }
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn item_is_public(item: &Item) -> bool {
    match item {
        Item::Struct(s) => is_public(&s.vis),
        Item::Enum(e) => is_public(&e.vis),
        Item::Trait(t) => is_public(&t.vis),
        Item::Type(t) => is_public(&t.vis),
        _ => false,
    }
}

fn matches_visibility_filter(vis: &Visibility, filter: VisibilityFilter) -> bool {
    match filter {
        VisibilityFilter::All => true,
        VisibilityFilter::PublicOnly => is_public(vis),
    }
}

fn item_matches_visibility_filter(item: &Item, filter: VisibilityFilter) -> bool {
    match filter {
        VisibilityFilter::All => true,
        VisibilityFilter::PublicOnly => item_is_public(item),
    }
}

fn generate_list_all(project: &Project, visibility: VisibilityFilter) -> Result<Output, String> {
    let mut output = String::new();

    // Group types by file
    let mut types_by_file: HashMap<String, Vec<Item>> = HashMap::new();
    for (_type_name, (file_path, item)) in &project.types {
        if item_matches_visibility_filter(item, visibility) {
            types_by_file
                .entry(file_path.clone())
                .or_default()
                .push(item.clone());
        }
    }

    // Group functions by file
    let mut funcs_by_file: HashMap<String, Vec<&Function>> = HashMap::new();
    for (name, func) in &project.functions {
        if matches_visibility_filter(&func.vis, visibility) {
            let file_path = find_file_for_function(name, project)
                .unwrap_or_else(|_| "<unknown>".to_string());
            funcs_by_file.entry(file_path).or_default().push(func);
        }
    }

    // Get all unique file paths and sort them
    let mut all_files: Vec<String> = types_by_file.keys()
        .chain(funcs_by_file.keys())
        .map(|s| s.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    all_files.sort();

    // Output types and functions grouped by file
    for file_path in all_files {
        output.push_str(&format!("=== {} ===\n", file_path));

        // Output types for this file
        if let Some(types) = types_by_file.get(&file_path) {
            for item in types {
                output.push_str(&format_type_item(item));
                output.push('\n');
            }
        }

        // Output functions for this file
        if let Some(funcs) = funcs_by_file.get_mut(&file_path) {
            // Sort functions by qualified name
            funcs.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
            for func in funcs {
                output.push_str(&format!("{}\n", func.signature()));
            }
        }
    }

    Ok(Output { content: output })
}

fn generate_call_graph_output(
    file_to_funcs: &HashMap<String, Vec<Function>>,
    file_to_types: &HashMap<String, Vec<Item>>,
    visibility: VisibilityFilter,
    root_func: Option<&str>,
) -> Result<Output, String> {
    let mut output = String::new();

    // Get all unique file paths and sort them
    let mut all_files: Vec<String> = file_to_types.keys()
        .chain(file_to_funcs.keys())
        .map(|s| s.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    all_files.sort();

    // Build a flat map of all reachable functions for easy lookup
    let mut all_funcs: HashMap<String, &Function> = HashMap::new();
    for functions in file_to_funcs.values() {
        for func in functions {
            all_funcs.insert(func.qualified_name.clone(), func);
        }
    }

    // Output types grouped by file
    for file_path in &all_files {
        if let Some(items) = file_to_types.get(file_path) {
            let filtered_items: Vec<_> = items.iter()
                .filter(|item| item_matches_visibility_filter(item, visibility))
                .collect();

            if !filtered_items.is_empty() {
                output.push_str(&format!("=== {} ===\n", file_path));
                for item in filtered_items {
                    output.push_str(&format_type_item(item));
                    output.push('\n');
                }
            }
        }
    }

    // If we have a root function, only show that function's tree
    if let Some(root_name) = root_func {
        if let Some(root_function) = all_funcs.get(root_name) {
            // Get the file for the root function
            let root_file = find_file_for_function(root_name, &Project {
                functions: all_funcs.iter().map(|(k, v)| (k.clone(), (*v).clone())).collect(),
                types: HashMap::new(),
            })?;

            output.push_str(&format!("=== {} ===\n", root_file));

            let mut visited_in_tree = HashSet::new();
            render_function_tree(root_function, &all_funcs, &mut visited_in_tree, 0, "", &mut output);
        }
    } else {
        // No root specified - show all functions as separate trees (old behavior)
        for file_path in &all_files {
            if let Some(functions) = file_to_funcs.get(file_path) {
                let mut funcs_to_show: Vec<_> = functions.iter()
                    .filter(|func| matches_visibility_filter(&func.vis, visibility))
                    .collect();

                if !funcs_to_show.is_empty() {
                    // Only print header if we haven't already (from types section)
                    if !file_to_types.contains_key(file_path) {
                        output.push_str(&format!("=== {} ===\n", file_path));
                    }

                    funcs_to_show.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));

                    for func in funcs_to_show {
                        let mut visited_in_tree = HashSet::new();
                        render_function_tree(func, &all_funcs, &mut visited_in_tree, 0, "", &mut output);
                        output.push('\n');
                    }
                }
            }
        }
    }

    Ok(Output { content: output })
}

fn render_function_tree(
    func: &Function,
    all_funcs: &HashMap<String, &Function>,
    visited_in_tree: &mut HashSet<String>,
    depth: usize,
    prefix: &str,
    output: &mut String,
) {
    // Print function signature
    if depth == 0 {
        output.push_str(&format!("{}\n", func.signature()));
    }

    visited_in_tree.insert(func.qualified_name.clone());

    // Get calls and filter to only project functions
    let calls = func.calls();
    let mut project_calls: Vec<(String, Option<String>)> = vec![];

    for call in &calls {
        // Try to resolve the call to a qualified name
        if let Some(qualified_name) = resolve_call_to_qualified(&call.name, all_funcs) {
            project_calls.push((qualified_name, call.context.clone()));
        }
    }

    // Render each call as a tree node
    for (i, (callee_qualified, context)) in project_calls.iter().enumerate() {
        let is_last = i == project_calls.len() - 1;
        let branch = if is_last { "└── " } else { "├── " };
        let extension = if is_last { "    " } else { "│   " };

        // Display name (strip file path for readability)
        let display_name = callee_qualified.split("::").last().unwrap_or(callee_qualified);

        if let Some(ctx) = context {
            output.push_str(&format!("{}{}{} [in: {}]", prefix, branch, display_name, ctx));
        } else {
            output.push_str(&format!("{}{}{}", prefix, branch, display_name));
        }

        // Check if already visited in this tree (cycle detection)
        if visited_in_tree.contains(callee_qualified) {
            output.push_str(" (already shown)\n");
        } else if let Some(callee_func) = all_funcs.get(callee_qualified) {
            output.push('\n');
            // Recursively render the callee's tree
            let new_prefix = format!("{}{}", prefix, extension);
            render_function_tree(callee_func, all_funcs, visited_in_tree, depth + 1, &new_prefix, output);
        } else {
            output.push('\n');
        }
    }
}

fn resolve_call_to_qualified(call_name: &str, all_funcs: &HashMap<String, &Function>) -> Option<String> {
    // Try exact match first
    if all_funcs.contains_key(call_name) {
        return Some(call_name.to_string());
    }

    // Try to find a function whose qualified name ends with ::call_name
    all_funcs.keys()
        .find(|qn| qn.ends_with(&format!("::{}", call_name)))
        .map(|s| s.clone())
}

// === HELPER FUNCTIONS (NO I/O) ===
fn format_type_item(item: &Item) -> String {
    match item {
        Item::Struct(s) => {
            let vis = visibility_to_string(&s.vis);
            let fields: Vec<(String, String)> = s
                .fields
                .iter()
                .map(|f| {
                    let vis_str = visibility_to_string(&f.vis);
                    let ty = format_type(&f.ty);
                    if let Some(ident) = &f.ident {
                        (format!("{}{}", vis_str, ident), ty)
                    } else {
                        (ty.clone(), ty)
                    }
                })
                .collect();

            let field_lines: Vec<String> = fields
                .iter()
                .map(|(name, ty)| format!("    {}: {}", name, ty))
                .collect();

            format!(
                "{}struct {} {{\n{}\n}}",
                vis,
                s.ident,
                field_lines.join(",\n")
            )
        }

        Item::Enum(e) => {
            let vis = visibility_to_string(&e.vis);
            let variants: Vec<String> = e
                .variants
                .iter()
                .map(|v| match &v.fields {
                    syn::Fields::Unit => format!("{}{}", vis, v.ident),
                    syn::Fields::Unnamed(fields) => {
                        let tys: Vec<String> =
                            fields.unnamed.iter().map(|f| format_type(&f.ty)).collect();
                        if tys.len() == 1 {
                            format!("{}({})", v.ident, tys[0])
                        } else {
                            let t = tys.join(", ");
                            format!("{}({})", v.ident, t)
                        }
                    }
                    syn::Fields::Named(fields) => {
                        let field_pairs: Vec<String> = fields
                            .named
                            .iter()
                            .map(|f| {
                                let vis_str = visibility_to_string(&f.vis);
                                let ty = format_type(&f.ty);
                                if let Some(ident) = &f.ident {
                                    format!("{}{}", vis_str, ident)
                                } else {
                                    ty.clone()
                                }
                            })
                            .collect();

                        let field_str = if field_pairs.len() == 1 {
                            format!("{}: {}", &field_pairs[0], format_type(&fields.named[0].ty))
                        } else {
                            field_pairs.join(", ")
                        };

                        format!("{}{{ {} }}", v.ident, field_str)
                    }
                })
                .collect();

            format!(
                "{}enum {} {{\n{}\n}}",
                vis,
                e.ident,
                variants
                    .iter()
                    .map(|v| format!("    {}", v))
                    .collect::<Vec<_>>()
                    .join(",\n")
            )
        }

        Item::Trait(t) => {
            let vis = visibility_to_string(&t.vis);
            let mut items: Vec<String> = vec![];

            for item in &t.items {
                match item {
                    syn::TraitItem::Fn(method) => {
                        let asyncness = if method.sig.asyncness.is_some() {
                            "async "
                        } else {
                            ""
                        };
                        let constness = if method.sig.constness.is_some() {
                            "const "
                        } else {
                            ""
                        };
                        let unsafety = if method.sig.unsafety.is_some() {
                            "unsafe "
                        } else {
                            ""
                        };

                        let args = format_args(&method.sig.inputs.iter().collect::<Vec<_>>());

                        let ret = match &method.sig.output {
                            syn::ReturnType::Default => String::new(),
                            syn::ReturnType::Type(_, ty) => format!(" -> {}", format_type(ty)),
                        };

                        items.push(format!(
                            "{}{}{}{}fn {}({}){};",
                            vis, asyncness, constness, unsafety, method.sig.ident, args, ret
                        ));
                    }
                    syn::TraitItem::Type(ty) => {
                        items.push(format!("type {};", ty.ident));
                    }
                    syn::TraitItem::Const(const_item) => {
                        items.push(format!(
                            "const {}: {};",
                            const_item.ident,
                            format_type(&const_item.ty)
                        ));
                    }
                    _ => {}
                }
            }

            if items.is_empty() {
                format!("{}trait {} {{\n}}", vis, t.ident)
            } else {
                let indented = items
                    .iter()
                    .map(|i| format!("    {}", i))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}trait {} {{\n{}\n}}", vis, t.ident, indented)
            }
        }

        Item::Type(t) => {
            let vis = visibility_to_string(&t.vis);
            let ty_str = match &*t.ty {
                syn::Type::Path(p) => p.path.to_token_stream().to_string(),
                _ => t.ty.to_token_stream().to_string(),
            };
            format!("{}type {} = {};", vis, t.ident, ty_str)
        }

        _ => unreachable!(),
    }
}

fn format_type(t: &Type) -> String {
    match t {
        Type::Path(p) => p.path.to_token_stream().to_string(),
        _ => t.to_token_stream().to_string(),
    }
}

fn visibility_to_string(vis: &Visibility) -> String {
    match vis {
        Visibility::Public(_) => "pub ",
        _ => "",
    }
    .to_string()
}

fn format_args(args: &[&FnArg]) -> String {
    args.iter()
        .map(|arg| match arg {
            FnArg::Receiver(_) => "self".to_string(),
            FnArg::Typed(pat_type) => pat_type.ty.to_token_stream().to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn find_file_for_function(qualified_name: &str, _project: &Project) -> Result<String, String> {
    // Extract file path from qualified_name (format: "file_path::function_name" or "file_path::Type::method")
    if let Some(first_separator) = qualified_name.find("::") {
        Ok(qualified_name[..first_separator].to_string())
    } else {
        Err(format!("Invalid qualified name format: {}", qualified_name))
    }
}

fn find_file_for_type(name: &str, project: &Project) -> Result<String, String> {
    for (type_name, (file_path, _)) in project.types.iter() {
        if type_name == name {
            return Ok(file_path.clone());
        }
    }
    Err(format!("Type {} not found in project", name))
}

fn collect_types_in_signature(sig: &syn::Signature, out: &mut HashSet<String>) {
    for arg in sig.inputs.iter() {
        if let FnArg::Typed(t) = arg {
            collect_types_in_type(&t.ty, out);
        }
    }

    match &sig.output {
        syn::ReturnType::Type(_, t) => collect_types_in_type(t, out),
        _ => {}
    }
}

fn collect_types_in_type(typ: &Type, out: &mut HashSet<String>) {
    match typ {
        Type::Path(p) => {
            if let Some(last_seg) = p.path.segments.last() {
                out.insert(last_seg.ident.to_string());
            }
        }

        Type::Reference(r) => collect_types_in_type(&r.elem, out),
        Type::Array(a) => collect_types_in_type(&a.elem, out),
        Type::Slice(s) => collect_types_in_type(&s.elem, out),

        _ => {}
    }
}

fn indent_block(block: &Block) -> String {
    let mut s = String::new();
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => {
                s.push_str(&format!("  {}\n", expr.to_token_stream().to_string()))
            }
            _ => {}
        }
    }
    s
}

fn extract_calls_from_block(block: &Block, out: &mut Vec<CallSite>) {
    for stmt in &block.stmts {
        match stmt {
            syn::Stmt::Expr(expr, _) => extract_calls_from_expr(&expr, out),
            _ => {}
        }
    }
}

fn extract_calls_from_expr(expr: &Expr, out: &mut Vec<CallSite>) {
    match expr {
        Expr::Call(call) => extract_path_ident(&call.func, out),
        Expr::MethodCall(method_call) => {
            let name = method_call.method.to_string();
            out.push(CallSite {
                name,
                context: None,
            });
        }
        Expr::Unary(unary) => extract_calls_from_expr(&unary.expr, out),
        Expr::Binary(binary) => {
            extract_calls_from_expr(&binary.left, out);
            extract_calls_from_expr(&binary.right, out);
        }
        Expr::Group(group) => extract_calls_from_expr(&group.expr, out),
        Expr::Block(block_expr) => {
            extract_calls_from_block(&block_expr.block, out);
        }
        Expr::If(i) => {
            let cond_str = i.cond.to_token_stream().to_string();
            extract_calls_from_expr(&i.cond, out);

            let mut then_calls = vec![];
            extract_calls_from_block(&i.then_branch, &mut then_calls);
            for mut call in then_calls {
                call.context = Some(format!("if ({})", cond_str));
                out.push(call);
            }

            if let Some((_, else_expr)) = &i.else_branch {
                match else_expr.as_ref() {
                    Expr::Block(block) => {
                        let mut else_calls = vec![];
                        extract_calls_from_block(&block.block, &mut else_calls);
                        for mut call in else_calls {
                            call.context = Some("else".to_string());
                            out.push(call);
                        }
                    }
                    other_expr => {
                        let mut else_calls = vec![];
                        extract_calls_from_expr(other_expr, &mut else_calls);
                        for mut call in else_calls {
                            call.context = Some("else".to_string());
                            out.push(call);
                        }
                    }
                };
            }
        }

        Expr::Match(m) => {
            extract_calls_from_expr(&m.expr, out);

            for arm in &m.arms {
                let pattern_str = arm.pat.to_token_stream().to_string();
                match arm.body.as_ref() {
                    Expr::Block(block) => {
                        let mut body_calls = vec![];
                        extract_calls_from_block(&block.block, &mut body_calls);
                        for mut call in body_calls {
                            call.context = Some(format!("match {}", pattern_str));
                            out.push(call);
                        }
                    }
                    other_expr => {
                        let mut body_calls = vec![];
                        extract_calls_from_expr(other_expr, &mut body_calls);
                        for mut call in body_calls {
                            call.context = Some(format!("match {}", pattern_str));
                            out.push(call);
                        }
                    }
                };
            }
        }

        Expr::Loop(l) => {
            extract_calls_from_block(&l.body, out);
        }

        Expr::While(w) => {
            let cond_str = w.cond.to_token_stream().to_string();
            extract_calls_from_expr(&w.cond, out);
            let mut body_calls = vec![];
            extract_calls_from_block(&w.body, &mut body_calls);
            for mut call in body_calls {
                call.context = Some(format!("while ({})", cond_str));
                out.push(call);
            }
        }

        Expr::ForLoop(f) => {
            let expr_str = f.expr.to_token_stream().to_string();
            extract_calls_from_expr(&f.expr, out);
            let mut body_calls = vec![];
            extract_calls_from_block(&f.body, &mut body_calls);
            for mut call in body_calls {
                call.context = Some(format!("for {}", expr_str));
                out.push(call);
            }
        }

        Expr::Async(a) => {
            extract_calls_from_block(&a.block, out);
        }

        Expr::Try(t) => {
            extract_calls_from_expr(&t.expr, out);
        }

        Expr::Macro(m) => {
            extract_path_from_syn_path(&m.mac.path, out);
        }

        Expr::Lit(_) | Expr::Const(_) => {}

        _ => {}
    }
}

fn extract_path_from_syn_path(path: &syn::Path, out: &mut Vec<CallSite>) {
    if let Some(last_seg) = path.segments.last() {
        out.push(CallSite {
            name: last_seg.ident.to_string(),
            context: None,
        });
    }
}

fn extract_path_ident(expr: &Expr, out: &mut Vec<CallSite>) {
    match expr {
        Expr::Path(p) => {
            if let Some(last_seg) = p.path.segments.last() {
                out.push(CallSite {
                    name: last_seg.ident.to_string(),
                    context: None,
                });
            }
        }

        Expr::MethodCall(m) => {
            out.push(CallSite {
                name: m.method.to_string(),
                context: None,
            });
        }

        _ => {}
    }
}
