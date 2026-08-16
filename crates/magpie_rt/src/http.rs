//! HTTP request context for Magpie handlers.
//!
//! Handlers can access the current HTTP request data via extern "C" functions
//! that read process-global environment variables set by the web server before
//! each handler invocation.
//!
//! Env var names:
//!   MAGPIE_HTTP_METHOD    — "GET", "POST", etc.
//!   MAGPIE_HTTP_PATH      — "/api/items"
//!   MAGPIE_HTTP_BODY      — raw request body string
//!   MAGPIE_HTTP_QUERY     — "page=1&limit=10"
//!
//! Environment variables are managed by the system libc and are process-global,
//! so they work correctly across the .so boundary: the web server sets them,
//! the handler .so reads them.

use crate::MpRtHeader;

extern "C" {
    fn getenv(name: *const i8) -> *const i8;
}

/// Read an env var and return a Magpie Str pointer (may be empty, never null).
unsafe fn env_to_str(name_bytes: &[u8]) -> *mut MpRtHeader {
    // Ensure null-terminated
    let name_cstr = match std::ffi::CString::new(name_bytes) {
        Ok(c) => c,
        Err(_) => return crate::mp_rt_str_from_utf8(std::ptr::null(), 0),
    };
    let ptr = getenv(name_cstr.as_ptr());
    if ptr.is_null() {
        return crate::mp_rt_str_from_utf8(std::ptr::null(), 0);
    }
    let cstr = std::ffi::CStr::from_ptr(ptr);
    let bytes = cstr.to_bytes();
    crate::mp_rt_str_from_utf8(bytes.as_ptr(), bytes.len() as u64)
}

/// Get the HTTP request method (e.g. "GET", "POST").
/// Returns a Magpie Str pointer.
#[no_mangle]
pub unsafe extern "C" fn mp_http_method() -> *mut MpRtHeader {
    env_to_str(b"MAGPIE_HTTP_METHOD\0")
}

/// Get the HTTP request path (e.g. "/api/items").
/// Returns a Magpie Str pointer.
#[no_mangle]
pub unsafe extern "C" fn mp_http_path() -> *mut MpRtHeader {
    env_to_str(b"MAGPIE_HTTP_PATH\0")
}

/// Get the HTTP request body.
/// Returns a Magpie Str pointer (may be empty).
#[no_mangle]
pub unsafe extern "C" fn mp_http_body() -> *mut MpRtHeader {
    env_to_str(b"MAGPIE_HTTP_BODY\0")
}

/// Get the HTTP query string (e.g. "page=1&limit=10").
/// Returns a Magpie Str pointer.
#[no_mangle]
pub unsafe extern "C" fn mp_http_query() -> *mut MpRtHeader {
    env_to_str(b"MAGPIE_HTTP_QUERY\0")
}

/// Set HTTP request env vars in the web server.
/// Must be called before each handler invocation.
pub fn set_request_env(method: &str, path: &str, body: &str, query: &str) {
    std::env::set_var("MAGPIE_HTTP_METHOD", method);
    std::env::set_var("MAGPIE_HTTP_PATH", path);
    std::env::set_var("MAGPIE_HTTP_BODY", body);
    std::env::set_var("MAGPIE_HTTP_QUERY", query);
}

/// Clear HTTP request env vars after handler completes.
pub fn clear_request_env() {
    std::env::remove_var("MAGPIE_HTTP_METHOD");
    std::env::remove_var("MAGPIE_HTTP_PATH");
    std::env::remove_var("MAGPIE_HTTP_BODY");
    std::env::remove_var("MAGPIE_HTTP_QUERY");
}