module std.http
exports { @method, @path, @body, @query }
imports { }
digest "0000000000000000000000000000000000000000000000000000000000000001"

extern "C" module http_rt {
  fn @method() -> Str attrs { link_name="mp_http_method" }
  fn @path() -> Str attrs { link_name="mp_http_path" }
  fn @body() -> Str attrs { link_name="mp_http_body" }
  fn @query() -> Str attrs { link_name="mp_http_query" }
}