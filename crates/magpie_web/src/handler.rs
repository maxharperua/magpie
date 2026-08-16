//! Magpie route handler compilation and execution.
//!
//! This module compiles Magpie route files (`.mp`) to shared libraries (`.so`)
//! and executes them at runtime, bridging HTTP requests to Magpie handler functions.

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

/// Compile a Magpie route file to a shared library and return the path to the .so.
pub fn compile_route_to_so(
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
        .map_err(|e| format!("failed to create build dir: {}", e))?;

    let route_dest = build_dir.join("index.mp");
    std::fs::copy(route_path, &route_dest)
        .map_err(|e| format!("failed to copy route file: {}", e))?;

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
        .map_err(|e| format!("failed to write Magpie.toml: {}", e))?;

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
            .map_err(|e| format!("failed to create build release dir: {}", e))?;
        std::fs::copy(&rt_release, build_release.join("libmagpie_rt.a"))
            .map_err(|e| format!("failed to copy libmagpie_rt.a: {}", e))?;
    }

    let output = Command::new("magpie")
        .args(["--emit", "shared-lib", "--profile", "release", "build"])
        .current_dir(&build_dir)
        .output()
        .map_err(|e| format!("failed to run magpie build: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("magpie build failed: {}", stderr));
    }
    // Find the produced .so file — name matches entry file name
        let so_name = format!("lib{}.so", route_name);
        let so_path = build_release.join(&so_name);
        if so_path.is_file() {
            Ok(so_path)
        } else {
            // Fallback: try libindex.so or libmain.so
            let fallbacks = ["libindex.so", "libmain.so"];
            for fb in &fallbacks {
                let fb_path = build_release.join(fb);
                if fb_path.is_file() {
                    return Ok(fb_path);
                }
            }
            Err(format!(
                "shared library not found at '{}' (searched for {:?})",
                so_path.display(),
                fallbacks
            ))
        }
}

/// Execute a compiled Magpie route handler.
///
/// Compiles the route file if needed (cache miss), then calls the handler function.
pub fn execute_route_handler(
    route_path: &Path,
    magpie_home: &Path,
    cache_dir: &Path,
) -> Result<i32, String> {
    let last_modified = std::fs::metadata(route_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let route_key = route_path.to_string_lossy().to_string();

    // Check cache
    {
        let cache =
            cache().lock().map_err(|e| format!("cache lock: {}", e))?;
        if let Some(entry) = cache.entries.get(&route_key) {
            if entry.last_modified == last_modified && entry.so_path.is_file() {
                return unsafe { call_handler(&entry.so_path) };
            }
        }
    }

    // Cache miss - compile
    let so_path = compile_route_to_so(route_path, magpie_home, cache_dir)?;

    let result = unsafe { call_handler(&so_path) };

    // Update cache
    if let Ok(mut cache) = cache().lock() {
        cache.entries.insert(
            route_key,
            CachedHandler {
                so_path,
                last_modified,
            },
        );
    }

    result
}

/// Load a .so and call the handler function.
unsafe fn call_handler(so_path: &Path) -> Result<i32, String> {
    let lib = Library::new(so_path)
        .map_err(|e| format!("failed to load .so '{}': {}", so_path.display(), e))?;

    // Try "main" first (when route file has @main as entry point)
    if let Ok(handler) = lib.get::<unsafe extern "C" fn() -> i32>(b"main") {
        let result = handler();
        // Don't forget the library — keep it alive while result is in use
        std::mem::forget(lib);
        return Ok(result);
    }

    // Try the mangled handler symbol: find it via sidecar file
    let sym_path = so_path.with_extension("sym");
    if let Ok(sym_name) = std::fs::read_to_string(&sym_path) {
        let sym_name = sym_name.trim();
        if let Ok(handler) = lib.get::<unsafe extern "C" fn() -> i32>(sym_name.as_bytes()) {
            let result = handler();
            std::mem::forget(lib);
            return Ok(result);
        }
    }

    // Fallback: try the first mp$0$FN$ symbol found via nm
    let output = std::process::Command::new("nm")
        .arg("-D")
        .arg(so_path)
        .output()
        .map_err(|e| format!("nm failed: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() >= 3 && parts[1] == "T" && parts[2].starts_with("mp$0$FN") {
            let sym_name = parts[2].to_string();
            // Cache the symbol name for next time
            let _ = std::fs::write(&sym_path, &sym_name);
            if let Ok(handler) = lib.get::<unsafe extern "C" fn() -> i32>(sym_name.as_bytes()) {
                let result = handler();
                std::mem::forget(lib);
                return Ok(result);
            }
        }
    }

    std::mem::forget(lib);
    Err(format!(
        "no handler symbol found in '{}'",
        so_path.display()
    ))
}

/// Clear the handler cache.
pub fn clear_handler_cache() {
    if let Ok(mut cache) = cache().lock() {
        cache.entries.clear();
    }
}