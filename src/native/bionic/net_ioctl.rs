use super::fd_table::FdEntry;

pub fn request(entry: &FdEntry, fd: i32, request: u32, argument: usize) -> Option<i64> {
    if !matches!(entry.kind, super::fd_table::FdKind::Socket) {
        return None;
    }
    super::net_if::request(fd, request, argument)
}
