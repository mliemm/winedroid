use super::abi_types::SSIZE_MAX;
use super::fatal::fortify_fatal;

const FD_SETSIZE: i32 = 1024;
const FD_SET_BYTES: u64 = 128;

pub fn check_count(function: &str, identifier: &str, value: u64) {
    // cite: bionic bionic_fortify.h#__check_count
    if value > SSIZE_MAX {
        fortify_fatal(&format!("{function}: {identifier} {value} > SSIZE_MAX"));
    }
}

pub fn check_access(function: &str, action: &str, claim: u64, actual: u64) {
    // cite: bionic bionic_fortify.h#__check_buffer_access
    if claim > actual {
        fortify_fatal(&format!("{function}: prevented {claim}-byte {action} {actual}-byte buffer"));
    }
}

pub fn check_fd_set(function: &str, fd: i32, set_size: u64) {
    // cite: bionic bionic_fortify.h#__check_fd_set
    if fd < 0 {
        fortify_fatal(&format!("{function}: file descriptor {fd} < 0"));
    }
    if fd >= FD_SETSIZE {
        fortify_fatal(&format!("{function}: file descriptor {fd} >= FD_SETSIZE {FD_SETSIZE}"));
    }
    if set_size < FD_SET_BYTES {
        fortify_fatal(&format!("{function}: set size {set_size} is too small to be an fd_set"));
    }
}
