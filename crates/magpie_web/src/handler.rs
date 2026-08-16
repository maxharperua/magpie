//! Magpie route handler compilation and execution.
//!
//! This module compiles Magpie route files (`.mp`) to shared libraries (`.so`)
//! and executes them via libloading, bridging HTTP requests to Magpie handler functions.
//!
//! # HTTP Method Routing
//!
//! Route files can define multiple handlers for different HTTP methods.
//! The Magpie compiler emits a `.manifest` JSON file alongside each `.so`
//! mapping Magpie function names (like `@get`, `@post`, `@handler`) to their
//! mangled ELF symbols.
//!
//! The dev server tries method-specific handlers first (`@get` for GET, `@post`
//! for POST, etc.), then falls back to `@handler`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::SystemTime;

use libloading::Library;

/// Cache of compiled route handlers.
struct HandlerCache {
    entries: HashMap<String, CachedHandler>,
}

struct CachedHandler {
    so_path: PathBuf,
    last_modified: SystemTime,
}

impl HandlerCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

fn cache() -> &'static Mutex<HandlerCache> {
    static CACHE: std::sync::OnceLock<Mutex<HandlerCache>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HandlerCache::new()))
}

/// Map an HTTP method to the Magpie function name it should try first.
fn method_to_handler_name(method: &str) -> &'static str {
    match method.to_ascii_uppercase().as_str() {
        "GET" => "@get",
        "POST" => "@post",
        "PUT" => "@put",
        "DELETE" => "@delete",
        "PATCH" => "@patch",
        "HEAD" => "@head",
        "OPTIONS" => "@options",
        _ => "@handler",
    }
}

/// Load the handler manifest JSON written by the Magpie compiler.
///
/// The compiler emits `<name>.manifest` alongside `<name>.so` containing a
/// JSON object like `{"@handler": "mp$0$FN$...", "@get": "mp$0$FN$..."}`.
fn load_handler_manifest(so_path: &Path) -> Result<HashMap<String, String>, String> {
    let manifest_path = so_path.with_extension("manifest");
    let raw = std::fs::read_to_string(&manifest_path).map_err(|e| {
        format!(
            "manifest not found at '{}': {e}",
            manifest_path.display()
        )
    })?;
    let parsed: HashMap<String, String> =
        serde_json::from_str(&raw).map_err(|e| format!("invalid manifest: {e}"))?;
    Ok(parsed)
}

/// Fallback: find the first `mp$0$FN$` symbol in a .so using `nm -D`.
///
/// Used when no compiler-generated manifest is available (older compiled routes).
fn find_first_handler_symbol(so_path: &Path) -> Result<String, String> {
    let output = Command::new("nm")
        .arg("-D")
        .arg(so_path)
        .output()
        .map_err(|e| format!("nm failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 3 && parts[1] == "T" {
            let name = parts[2];
            if name.contains("mp$0$FN$") && !name.contains("gpu") {
                return Ok(name.to_string());
            }
        }
    }

    Err(format!(
        "no handler symbol found in '{}'",
        so_path.display()
    ))
}

/// Compile a Magpie route file to a shared library.
fn compile_route_to_so(
    route_path: &Path,
    magpie_home: &Path,
    out_dir: &Path,
) -> Result<PathBuf, String> {
    let route_name = route_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("handler");

    let build_dir = out_dir.join(format!("route_{}", route_name));
    std::fs::create_dir_all(&build_dir)
        .map_err(|e| format!("failed to create build dir: {e}"))?;

    let route_dest = build_dir.join("index.mp");
    std::fs::copy(route_path, &route_dest)
        .map_err(|e| format!("failed to copy route file: {e}"))?;

    let std_path = magpie_home.join("std").to_string_lossy().to_string();
    let magpie_toml = format!(
        r#"[package]
name = "route_{name}"
version = "0.1.0"
edition = "2026"

[build]
entry = "index.mp"
profile_default = "release"

[dependencies]
std = {{ path = "{std_path}" }}
"#,
        name = route_name,
        std_path = std_path
    );
    std::fs::write(build_dir.join("Magpie.toml"), &magpie_toml)
        .map_err(|e| format!("failed to write Magpie.toml: {e}"))?;

    let rt_release = magpie_home
        .join("target")
        .join("release")
        .join("libmagpie_rt.a");
    let build_release = build_dir
        .join("target")
        .join("x86_64-unknown-linux")
        .join("release");
    if rt_release.is_file() {
        std::fs::create_dir_all(&build_release)
            .map_err(|e| format!("failed to create build release dir: {e}"))?;
        std::fs::copy(&rt_release, build_release.join("libmagpie_rt.a"))
            .map_err(|e| format!("failed to copy libmagpie_rt.a: {e}"))?;
    }

    let output = Command::new("magpie")
        .args(["--emit", "shared-lib", "--profile", "release", "build"])
        .current_dir(&build_dir)
        .output()
        .map_err(|e| format!("failed to run magpie build: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("magpie build failed: {stderr}"));
    }

    // Find the produced .so file
    let so_name = format!("lib{route_name}.so");
    let so_path = build_release.join(&so_name);
    let so_path = if so_path.is_file() {
        so_path
    } else {
        let fallbacks = ["libindex.so", "libmain.so"];
        let mut found = None;
        for fb in &fallbacks {
            let fb_path = build_release.join(fb);
            if fb_path.is_file() {
                found = Some(fb_path);
                break;
            }
        }
        found.ok_or_else(|| {
            format!(
                "shared library not found at '{}'",
                so_path.display()
            )
        })?
    };

    Ok(so_path)
}

/// Execute a compiled Magpie route handler.
///
/// Compiles the `.mp` route file if needed, then loads the shared library
/// and calls the correct method-specific function.
///
/// Sets process-global env vars (`MAGPIE_HTTP_[METHOD|PATH|BODY|QUERY]`) so the
/// handler can access request data via `std.http` functions.
pub fn execute_route_handler(
    route_path: &Path,
    magpie_home: &Path,
    cache_dir: &Path,
    method: &str,
    path: &str,
    body: &str,
    query: &str,
) -> Result<i32, String> {
    // Set process-global env vars so the handler .so can read request data.
    std::env::set_var("MAGPIE_HTTP_METHOD", method);
    std::env::set_var("MAGPIE_HTTP_PATH", path);
    std::env::set_var("MAGPIE_HTTP_BODY", body);
    std::env::set_var("MAGPIE_HTTP_QUERY", query);

    let last_modified = std::fs::metadata(route_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let route_key = format!("{}:{}", route_path.display(), method);

    // Check cache
    {
        let cache_obj = cache().lock().map_err(|e| format!("cache lock: {e}"))?;
        if let Some(entry) = cache_obj.entries.get(&route_key) {
            if entry.last_modified == last_modified && entry.so_path.is_file() {
                return unsafe { call_handler(&entry.so_path, method) };
            }
        }
    }

    // Cache miss — compile
    let so_path = compile_route_to_so(route_path, magpie_home, cache_dir)?;
    let result = unsafe { call_handler(&so_path, method) };

    // Update cache
    if let Ok(mut cache_obj) = cache().lock() {
        cache_obj.entries.insert(
            route_key,
            CachedHandler {
                so_path,
                last_modified,
            },
        );
    }

    result
}

/// Load a .so and call the handler for the given HTTP method.
///
/// 1. Tries the compiler-generated `.manifest` file for method-specific dispatch.
/// 2. Falls back to `nm -D` first-symbol lookup for older routes.
unsafe fn call_handler(so_path: &Path, method: &str) -> Result<i32, String> {
    // Determine which function names to try
    let target_name = method_to_handler_name(method);
    let try_names: &[&str] = if target_name == "@handler" {
        &["@handler"]
    } else {
        // Per method: try method-specific first, then fallback
        if method.to_ascii_uppercase().as_str() == "GET" {
            &["@get", "@handler"]
        } else if method.to_ascii_uppercase().as_str() == "POST" {
            &["@post", "@handler"]
        } else if method.to_ascii_uppercase().as_str() == "PUT" {
            &["@put", "@handler"]
        } else if method.to_ascii_uppercase().as_str() == "DELETE" {
            &["@delete", "@handler"]
        } else {
            &["@handler"]
        }
    };

    // Try compiler manifest first
    if let Ok(manifest) = load_handler_manifest(so_path) {
        let lib = Library::new(so_path)
            .map_err(|e| format!("failed to load .so '{}': {e}", so_path.display()))?;

        // 1. Run middleware chain (all @middleware_* functions)
        let mut middleware_fns: Vec<&String> = manifest.keys()
            .filter(|k| k.starts_with("@middleware_"))
            .collect();
        middleware_fns.sort();

        for mw_name in &middleware_fns {
            if let Some(mangled) = manifest.get(*mw_name) {
                if let Ok(func) = lib.get::<unsafe extern "C" fn() -> i32>(mangled.as_bytes()) {
                    let status = func();
                    if status != 200 {
                        std::mem::forget(lib);
                        return Ok(status);
                    }
                }
            }
        }

        // 2. Run main handler (method-specific, then fallback)
        for name in try_names {
            if let Some(mangled) = manifest.get(*name) {
                if let Ok(func) = lib.get::<unsafe extern "C" fn() -> i32>(mangled.as_bytes()) {
                    let result = func();
                    std::mem::forget(lib);
                    return Ok(result);
                }
            }
        }

        std::mem::forget(lib);
        return Err(format!(
            "no handler '{}' found in manifest for '{}'",
            try_names.join("', '"),
            so_path.display()
        ));
    }

    // Fallback: no manifest — find first handler symbol via nm -D
    let symbol = find_first_handler_symbol(so_path)?;
    let lib = Library::new(so_path)
        .map_err(|e| format!("failed to load .so '{}': {e}", so_path.display()))?;
    let func = lib
        .get::<unsafe extern "C" fn() -> i32>(symbol.as_bytes())
        .map_err(|e| format!("symbol lookup failed: {e}"))?;
    let result = func();
    std::mem::forget(lib);
    Ok(result)
}

/// Clear the handler cache.
pub fn clear_handler_cache() {
    if let Ok(mut cache_obj) = cache().lock() {
        cache_obj.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_to_handler_name() {
        assert_eq!(method_to_handler_name("GET"), "@get");
        assert_eq!(method_to_handler_name("POST"), "@post");
        assert_eq!(method_to_handler_name("PUT"), "@put");
        assert_eq!(method_to_handler_name("DELETE"), "@delete");
        assert_eq!(method_to_handler_name("PATCH"), "@patch");
        assert_eq!(method_to_handler_name("get"), "@get");
        assert_eq!(method_to_handler_name("Get"), "@get");
        assert_eq!(method_to_handler_name("UNKNOWN"), "@handler");
    }
}