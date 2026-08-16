//! SQLite FFI bridge for Magpie runtime.

use std::collections::HashMap;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::{mp_rt_str_bytes, mp_rt_str_from_utf8, MpRtHeader};

// Global DB connection registry.
static DB_CONNECTIONS: Mutex<Option<HashMap<i64, Connection>>> = Mutex::new(None);
static NEXT_HANDLE: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

/// Open a SQLite database. Returns a positive i64 handle, or 0 on error.
#[no_mangle]
pub unsafe extern "C" fn mp_db_open(path: *mut MpRtHeader) -> i64 {
    let mut len: u64 = 0;
    let bytes = mp_rt_str_bytes(path, &mut len);
    if bytes.is_null() {
        return 0;
    }
    let path_str = match std::str::from_utf8(std::slice::from_raw_parts(bytes, len as usize)) {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match Connection::open(path_str) {
        Ok(conn) => {
            let handle = NEXT_HANDLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let mut guard = DB_CONNECTIONS.lock().unwrap();
            let map = guard.get_or_insert_with(HashMap::new);
            map.insert(handle, conn);
            handle
        }
        Err(_) => 0,
    }
}

/// Execute a SQL statement. Returns SQLITE_OK (0) on success, or the error code.
#[no_mangle]
pub unsafe extern "C" fn mp_db_exec(db: i64, sql: *mut MpRtHeader) -> i32 {
    let mut len: u64 = 0;
    let bytes = mp_rt_str_bytes(sql, &mut len);
    if bytes.is_null() {
        return -1;
    }
    let sql_str = match std::str::from_utf8(std::slice::from_raw_parts(bytes, len as usize)) {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let guard = DB_CONNECTIONS.lock().unwrap();
    let map = match guard.as_ref() {
        Some(m) => m,
        None => return -1,
    };
    let conn = match map.get(&db) {
        Some(c) => c,
        None => return -1,
    };

    match conn.execute_batch(sql_str) {
        Ok(_) => 0,
        Err(e) => e.sqlite_error().map(|e| e.code as i32).unwrap_or(1),
    }
}

/// Query the database and return results as a JSON string.
/// Returns a Magpie Str object, or null on error.
#[no_mangle]
pub unsafe extern "C" fn mp_db_query_json(db: i64, sql: *mut MpRtHeader) -> *mut MpRtHeader {
    let mut len: u64 = 0;
    let bytes = mp_rt_str_bytes(sql, &mut len);
    if bytes.is_null() {
        return std::ptr::null_mut();
    }
    let sql_str = match std::str::from_utf8(std::slice::from_raw_parts(bytes, len as usize)) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let guard = DB_CONNECTIONS.lock().unwrap();
    let map = match guard.as_ref() {
        Some(m) => m,
        None => return std::ptr::null_mut(),
    };
    let conn = match map.get(&db) {
        Some(c) => c,
        None => return std::ptr::null_mut(),
    };

    let mut stmt = match conn.prepare(sql_str) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let col_count = stmt.column_count();
    let col_names: Vec<String> = (0..col_count)
        .map(|i| stmt.column_name(i).unwrap_or("?").to_string())
        .collect();

    let rows_result: Result<Vec<Vec<rusqlite::types::Value>>, _> = stmt
        .query_map([], |row| {
            let mut row_values = Vec::new();
            for i in 0..col_count {
                let val = row.get::<_, rusqlite::types::Value>(i)?;
                row_values.push(val);
            }
            Ok(row_values)
        })
        .and_then(|iter| iter.collect::<Result<Vec<_>, _>>());

    let rows = match rows_result {
        Ok(r) => r,
        Err(_) => return std::ptr::null_mut(),
    };

    // Build JSON string
    let mut json = String::from("[");
    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx > 0 {
            json.push(',');
        }
        json.push('{');
        for (col_idx, val) in row.iter().enumerate() {
            if col_idx > 0 {
                json.push(',');
            }
            json.push('"');
            json.push_str(&col_names[col_idx]);
            json.push_str("\":");
            match val {
                rusqlite::types::Value::Null => json.push_str("null"),
                rusqlite::types::Value::Integer(i) => json.push_str(&i.to_string()),
                rusqlite::types::Value::Real(f) => json.push_str(&f.to_string()),
                rusqlite::types::Value::Text(t) => {
                    json.push('"');
                    for ch in t.chars() {
                        match ch {
                            '"' => json.push_str("\\\""),
                            '\\' => json.push_str("\\\\"),
                            '\n' => json.push_str("\\n"),
                            '\r' => json.push_str("\\r"),
                            '\t' => json.push_str("\\t"),
                            c => json.push(c),
                        }
                    }
                    json.push('"');
                }
                rusqlite::types::Value::Blob(_) => json.push_str("\"[blob]\""),
            }
        }
        json.push('}');
    }
    json.push(']');

    mp_rt_str_from_utf8(json.as_ptr(), json.len() as u64)
}

/// Close a database connection. Returns 0 on success.
#[no_mangle]
pub unsafe extern "C" fn mp_db_close(db: i64) -> i32 {
    let mut guard = DB_CONNECTIONS.lock().unwrap();
    let map = match guard.as_mut() {
        Some(m) => m,
        None => return -1,
    };
    match map.remove(&db) {
        Some(_) => 0,
        None => -1,
    }
}