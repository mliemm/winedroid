use super::super::bridge::{register_data_in, register_handler_in, CallContext};

pub const LIBC: &str = "libc.so";
pub const LIBM: &str = "libm.so";
pub const LIBDL: &str = "libdl.so";
pub const LIBLOG: &str = "liblog.so";
pub const LIBSTDCXX: &str = "libstdc++.so";

pub fn in_libc(name: &str, handler: impl Fn(&mut CallContext) + Send + Sync + 'static) {
    register_handler_in(LIBC, name, handler);
}

pub fn in_libm(name: &str, handler: impl Fn(&mut CallContext) + Send + Sync + 'static) {
    register_handler_in(LIBM, name, handler);
}

pub fn in_libdl(name: &str, handler: impl Fn(&mut CallContext) + Send + Sync + 'static) {
    register_handler_in(LIBDL, name, handler);
}

pub fn in_liblog(name: &str, handler: impl Fn(&mut CallContext) + Send + Sync + 'static) {
    register_handler_in(LIBLOG, name, handler);
}

pub fn in_libstdcxx(name: &str, handler: impl Fn(&mut CallContext) + Send + Sync + 'static) {
    register_handler_in(LIBSTDCXX, name, handler);
}

pub fn data_in_libstdcxx(name: &str, bytes: usize) -> usize {
    register_data_in(LIBSTDCXX, name, bytes)
}

pub fn data_in_libc(name: &str, bytes: usize) -> usize {
    register_data_in(LIBC, name, bytes)
}

pub fn data_in_libm(name: &str, bytes: usize) -> usize {
    register_data_in(LIBM, name, bytes)
}
