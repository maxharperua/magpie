//! Built-in API handler for the Magpie dev server.
//!
//! Handles `/api/*` routes in Rust, bypassing the Magpie compiler
//! (which has a known issue with extern "C" module codegen).
//!
//! # Endpoints
//! - `GET /api/items` — list all items as JSON
//! - `GET /api/items/{id}` — get one item
//! - `POST /api/items` — create item (JSON: {name, value})
//! - `DELETE /api/items/{id}` — delete item

use std::path::Path;

use rusqlite::Connection;

/// Parse the request path to extract endpoint and params.
/// e.g. `/api/items` -> (`items`, None), `/api/items/42` -> (`items`, Some(42))
fn parse_api_path(path: &str) -> Option<(&str, Option<i64>)> {
    let cleaned = path.trim_start_matches('/');
    let parts: Vec<&str> = cleaned.split('/').collect();
    match parts.as_slice() {
        ["api", "items"] => Some(("items", None)),
        ["api", "items", id] => {
            let id: i64 = id.parse().ok()?;
            Some(("items", Some(id)))
        }
        ["api"] => Some(("root", None)),
        _ => None,
    }
}

fn ensure_db(app_dir: &Path) -> Result<Connection, String> {
    let db_path = app_dir.join("app_data.db");
    Connection::open(&db_path).map_err(|e| format!("DB open error: {}", e))
}

fn json_err(msg: &str) -> Vec<u8> {
    format!("{{\"error\":\"{}\"}}", msg).into_bytes()
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Handle an API request. Returns (status_line, content_type, body_bytes).
pub fn handle_api_request(
    method: &str,
    request_path: &str,
    body: &[u8],
    app_dir: &Path,
) -> (String, String, Vec<u8>) {
    let parsed = parse_api_path(request_path);
    let (endpoint, param) = match parsed {
        Some(p) => p,
        None => {
            return (
                "404 Not Found".to_string(),
                "application/json".to_string(),
                json_err("unknown endpoint"),
            );
        }
    };

    let conn = match ensure_db(app_dir) {
        Ok(c) => c,
        Err(e) => {
            return (
                "500 Internal Server Error".to_string(),
                "application/json".to_string(),
                json_err(&e),
            );
        }
    };

    match (method, endpoint, param) {
        ("GET", "items", None) => handle_list_items(&conn),
        ("GET", "items", Some(id)) => handle_get_item(&conn, id),
        ("POST", "items", None) => handle_create_item(&conn, body),
        ("DELETE", "items", Some(id)) => handle_delete_item(&conn, id),
        ("GET", "root", None) => (
            "200 OK".to_string(),
            "application/json".to_string(),
            br#"{"status":"ok","version":"0.1.0-api"}"#.to_vec(),
        ),
        _ => (
            "405 Method Not Allowed".to_string(),
            "application/json".to_string(),
            json_err("method not allowed"),
        ),
    }
}

fn ensure_items_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            value TEXT NOT NULL DEFAULT ''
        )",
    )
    .map_err(|e| format!("migration: {}", e))
}

fn handle_list_items(conn: &Connection) -> (String, String, Vec<u8>) {
    if let Err(e) = ensure_items_table(conn) {
        return (
            "500 Internal Server Error".to_string(),
            "application/json".to_string(),
            json_err(&e),
        );
    }

    let mut stmt = match conn.prepare("SELECT id, name, value FROM items ORDER BY id") {
        Ok(s) => s,
        Err(e) => {
            return (
                "500 Internal Server Error".to_string(),
                "application/json".to_string(),
                json_err(&format!("query prep: {}", e)),
            );
        }
    };

    let rows: Result<Vec<(i64, String, String)>, _> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .and_then(|iter| iter.collect());

    match rows {
        Ok(rows) => {
            let mut json = String::from("[");
            for (i, (id, name, value)) in rows.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                json.push_str(&format!(
                    "{{\"id\":{},\"name\":\"{}\",\"value\":\"{}\"}}",
                    id,
                    escape_json(name),
                    escape_json(value)
                ));
            }
            json.push(']');
            ("200 OK".to_string(), "application/json".to_string(), json.into_bytes())
        }
        Err(e) => (
            "500 Internal Server Error".to_string(),
            "application/json".to_string(),
            json_err(&format!("query exec: {}", e)),
        ),
    }
}

fn handle_get_item(conn: &Connection, id: i64) -> (String, String, Vec<u8>) {
    if let Err(e) = ensure_items_table(conn) {
        return (
            "500 Internal Server Error".to_string(),
            "application/json".to_string(),
            json_err(&e),
        );
    }

    let mut stmt = match conn.prepare("SELECT id, name, value FROM items WHERE id = ?1") {
        Ok(s) => s,
        Err(e) => {
            return (
                "500 Internal Server Error".to_string(),
                "application/json".to_string(),
                json_err(&format!("query prep: {}", e)),
            );
        }
    };

    let result: Result<Option<(i64, String, String)>, _> = stmt
        .query_map([id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .and_then(|iter| iter.collect::<Result<Vec<_>, _>>())
        .map(|v| v.into_iter().next());

    match result {
        Ok(Some((id, name, value))) => (
            "200 OK".to_string(),
            "application/json".to_string(),
            format!("{{\"id\":{},\"name\":\"{}\",\"value\":\"{}\"}}", id, escape_json(&name), escape_json(&value)).into_bytes(),
        ),
        Ok(None) => (
            "404 Not Found".to_string(),
            "application/json".to_string(),
            format!("{{\"error\":\"item {} not found\"}}", id).into_bytes(),
        ),
        Err(e) => (
            "500 Internal Server Error".to_string(),
            "application/json".to_string(),
            json_err(&format!("query exec: {}", e)),
        ),
    }
}

fn handle_create_item(conn: &Connection, body: &[u8]) -> (String, String, Vec<u8>) {
    let parsed: serde_json::Value = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "400 Bad Request".to_string(),
                "application/json".to_string(),
                json_err(&format!("invalid JSON: {}", e)),
            );
        }
    };

    let name = match parsed.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => {
            return (
                "400 Bad Request".to_string(),
                "application/json".to_string(),
                json_err("missing 'name' field"),
            );
        }
    };

    let value = parsed.get("value").and_then(|v| v.as_str()).unwrap_or("");

    if let Err(e) = ensure_items_table(conn) {
        return (
            "500 Internal Server Error".to_string(),
            "application/json".to_string(),
            json_err(&e),
        );
    }

    match conn.execute("INSERT INTO items (name, value) VALUES (?1, ?2)", [name, value]) {
        Ok(_) => {
            let id = conn.last_insert_rowid();
            (
                "201 Created".to_string(),
                "application/json".to_string(),
                format!("{{\"id\":{},\"name\":\"{}\",\"value\":\"{}\"}}", id, escape_json(name), escape_json(value)).into_bytes(),
            )
        }
        Err(e) => (
            "500 Internal Server Error".to_string(),
            "application/json".to_string(),
            json_err(&format!("insert: {}", e)),
        ),
    }
}

fn handle_delete_item(conn: &Connection, id: i64) -> (String, String, Vec<u8>) {
    if let Err(e) = ensure_items_table(conn) {
        return (
            "500 Internal Server Error".to_string(),
            "application/json".to_string(),
            json_err(&e),
        );
    }

    match conn.execute("DELETE FROM items WHERE id = ?1", [id]) {
        Ok(count) if count > 0 => (
            "200 OK".to_string(),
            "application/json".to_string(),
            format!("{{\"deleted\":{}}}", id).into_bytes(),
        ),
        Ok(_) => (
            "404 Not Found".to_string(),
            "application/json".to_string(),
            format!("{{\"error\":\"item {} not found\"}}", id).into_bytes(),
        ),
        Err(e) => (
            "500 Internal Server Error".to_string(),
            "application/json".to_string(),
            json_err(&format!("delete: {}", e)),
        ),
    }
}