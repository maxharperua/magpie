; ModuleID = 'gpu.ptx_hip'
source_filename = "gpu.ptx_hip"

@"mp$0$ABI$generics_mode" = weak_odr constant i8 0
declare void @mp_rt_init()
declare void @mp_rt_gpu_init()
declare void @mp_gpu_register_all_kernels()
declare ptr @mp_rt_alloc(i32, i64, i64, i32)
declare void @mp_rt_register_types(ptr, i32)
declare void @mp_rt_retain_strong(ptr)
declare void @mp_rt_release_strong(ptr)
declare void @mp_rt_retain_weak(ptr)
declare void @mp_rt_release_weak(ptr)
declare ptr @mp_rt_weak_upgrade(ptr)
declare void @mp_rt_panic(ptr) noreturn
declare ptr @mp_rt_arr_new(i32, i64, i64)
declare i64 @mp_rt_arr_len(ptr)
declare ptr @mp_rt_arr_get(ptr, i64)
declare void @mp_rt_arr_set(ptr, i64, ptr, i64)
declare void @mp_rt_arr_push(ptr, ptr, i64)
declare i32 @mp_rt_arr_pop(ptr, ptr, i64)
declare ptr @mp_rt_arr_slice(ptr, i64, i64)
declare i32 @mp_rt_arr_contains(ptr, ptr, i64, ptr)
declare void @mp_rt_arr_sort(ptr, ptr)
declare void @mp_rt_arr_foreach(ptr, ptr)
declare ptr @mp_rt_arr_map(ptr, ptr, i32, i64)
declare ptr @mp_rt_arr_filter(ptr, ptr)
declare void @mp_rt_arr_reduce(ptr, ptr, i64, ptr)
declare ptr @mp_rt_callable_new(ptr, ptr)
declare ptr @mp_rt_callable_fn_ptr(ptr)
declare ptr @mp_rt_callable_data_ptr(ptr)
declare i64 @mp_rt_callable_capture_size(ptr)
declare ptr @mp_rt_map_new(i32, i32, i64, i64, i64, ptr, ptr)
declare i64 @mp_rt_map_len(ptr)
declare ptr @mp_rt_map_get(ptr, ptr, i64)
declare void @mp_rt_map_set(ptr, ptr, i64, ptr, i64)
declare i32 @mp_rt_map_take(ptr, ptr, i64, ptr, i64)
declare i32 @mp_rt_map_delete(ptr, ptr, i64)
declare i32 @mp_rt_map_contains_key(ptr, ptr, i64)
declare ptr @mp_rt_map_keys(ptr)
declare ptr @mp_rt_map_values(ptr)
declare ptr @mp_rt_str_concat(ptr, ptr)
declare i64 @mp_rt_str_len(ptr)
declare i32 @mp_rt_str_eq(ptr, ptr)
declare i32 @mp_rt_str_cmp(ptr, ptr)
declare ptr @mp_rt_str_slice(ptr, i64, i64)
declare ptr @mp_rt_str_bytes(ptr, ptr)
declare ptr @mp_rt_str_from_utf8(ptr, i64)
declare i64 @mp_std_hash_str(ptr)
declare i64 @mp_rt_bytes_hash(ptr, i64)
declare i32 @mp_rt_bytes_eq(ptr, ptr, i64)
declare i32 @mp_rt_bytes_cmp(ptr, ptr, i64)
declare i32 @mp_rt_json_try_encode(ptr, i32, ptr, ptr)
declare i32 @mp_rt_json_try_decode(ptr, i32, ptr, ptr)
declare i32 @mp_rt_json_decoded_free(ptr, i32)
declare i32 @mp_rt_str_try_parse_i64(ptr, ptr, ptr)
declare i32 @mp_rt_str_try_parse_u64(ptr, ptr, ptr)
declare i32 @mp_rt_str_try_parse_f64(ptr, ptr, ptr)
declare i32 @mp_rt_str_try_parse_bool(ptr, ptr, ptr)
declare ptr @mp_rt_strbuilder_new()
declare void @mp_rt_strbuilder_append_str(ptr, ptr)
declare void @mp_rt_strbuilder_append_i64(ptr, i64)
declare void @mp_rt_strbuilder_append_i32(ptr, i32)
declare void @mp_rt_strbuilder_append_f64(ptr, double)
declare void @mp_rt_strbuilder_append_bool(ptr, i32)
declare ptr @mp_rt_strbuilder_build(ptr)
declare i32 @mp_rt_future_poll(ptr)
declare void @mp_rt_future_take(ptr, ptr)
declare i64 @mp_rt_gpu_buffer_len(ptr)
declare i32 @mp_rt_gpu_buffer_read(ptr, i64, ptr, i64)
declare i32 @mp_rt_gpu_buffer_write(ptr, i64, ptr, i64)
declare i32 @mp_rt_gpu_launch_sync(ptr, i64, i32, i32, i32, i32, i32, i32, ptr, i64, ptr)
declare i32 @mp_rt_gpu_launch_async(ptr, i64, i32, i32, i32, i32, i32, i32, ptr, i64, ptr, ptr)
declare { i8, i1 } @llvm.sadd.with.overflow.i8(i8, i8)
declare { i8, i1 } @llvm.ssub.with.overflow.i8(i8, i8)
declare { i8, i1 } @llvm.smul.with.overflow.i8(i8, i8)
declare { i16, i1 } @llvm.sadd.with.overflow.i16(i16, i16)
declare { i16, i1 } @llvm.ssub.with.overflow.i16(i16, i16)
declare { i16, i1 } @llvm.smul.with.overflow.i16(i16, i16)
declare { i32, i1 } @llvm.sadd.with.overflow.i32(i32, i32)
declare { i32, i1 } @llvm.ssub.with.overflow.i32(i32, i32)
declare { i32, i1 } @llvm.smul.with.overflow.i32(i32, i32)
declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.ssub.with.overflow.i64(i64, i64)
declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)
declare { i128, i1 } @llvm.sadd.with.overflow.i128(i128, i128)
declare { i128, i1 } @llvm.ssub.with.overflow.i128(i128, i128)
declare { i128, i1 } @llvm.smul.with.overflow.i128(i128, i128)


define internal void @mp$0$INIT_TYPES$FXBTX503S2() {
entry:
  ret void
}

define void @mp$0$FN$SXBR0XVANJ() {
bb0:
  ret void
}

define void @mp$0$FN$3129YENZP0() {
bb0:
  ret void
}

define i32 @mp$0$FN$JRNCVHAHAQ() {
bb0:
  ret i32 0
}

define i32 @main(i32 %argc, ptr %argv) {
entry:
  call void @mp_rt_init()
  call void @mp$0$INIT_TYPES$FXBTX503S2()
  call void @mp_gpu_register_all_kernels()
  %ret = call i32 @mp$0$FN$JRNCVHAHAQ()
  ret i32 %ret
}
