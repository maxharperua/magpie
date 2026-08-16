module std.db
exports { @open, @exec, @query_json, @close }
imports { }
digest "0000000000000000000000000000000000000000000000000000000000000001"

extern "C" module db_rt {
  fn @open(%path: Str) -> i64 attrs { link_name="mp_db_open" }
  fn @exec(%db: i64, %sql: Str) -> i32 attrs { link_name="mp_db_exec" }
  fn @query_json(%db: i64, %sql: Str) -> Str attrs { link_name="mp_db_query_json" }
  fn @close(%db: i64) -> i32 attrs { link_name="mp_db_close" }
}