use super::linux_abi::{at, fcntl, mman, open, seek, socket};

pub const DARWIN_F_NOCACHE: i32 = 48;
pub const DARWIN_F_FULLFSYNC: i32 = 51;

const TRANSLATED_OPEN_BITS: &[(i32, i32)] = &[
    (open::CREAT, libc::O_CREAT),
    (open::EXCL, libc::O_EXCL),
    (open::NOCTTY, libc::O_NOCTTY),
    (open::TRUNC, libc::O_TRUNC),
    (open::APPEND, libc::O_APPEND),
    (open::NONBLOCK, libc::O_NONBLOCK),
    (open::FASYNC, libc::O_ASYNC),
    (open::DIRECTORY, libc::O_DIRECTORY),
    (open::NOFOLLOW, libc::O_NOFOLLOW),
    (open::CLOEXEC, libc::O_CLOEXEC),
];

pub fn open_flags_to_host(linux: i32) -> i32 {
    // cite: bionic fcntl.h#O_CREAT, O_DIRECTORY
    let mut host = linux & open::ACCMODE;
    for (from, to) in TRANSLATED_OPEN_BITS {
        if linux & from != 0 {
            host |= to;
        }
    }
    if linux & open::SYNC == open::SYNC {
        host |= libc::O_SYNC;
    } else if linux & open::DSYNC != 0 {
        host |= libc::O_DSYNC;
    }
    host
}

pub fn status_flags_from_host(host: i32) -> i32 {
    let mut linux = 0;
    for (to, from) in [(open::APPEND, libc::O_APPEND), (open::NONBLOCK, libc::O_NONBLOCK), (open::FASYNC, libc::O_ASYNC)] {
        if host & from != 0 {
            linux |= to;
        }
    }
    linux
}

pub fn status_flags_to_host(linux: i32) -> i32 {
    let mut host = 0;
    for (from, to) in [(open::APPEND, libc::O_APPEND), (open::NONBLOCK, libc::O_NONBLOCK), (open::FASYNC, libc::O_ASYNC)] {
        if linux & from != 0 {
            host |= to;
        }
    }
    host
}

pub fn seek_to_host(whence: i32) -> Option<i32> {
    // cite: bionic seek_constants.h#SEEK_DATA, SEEK_HOLE
    match whence {
        seek::SET => Some(libc::SEEK_SET),
        seek::CUR => Some(libc::SEEK_CUR),
        seek::END => Some(libc::SEEK_END),
        seek::DATA => Some(libc::SEEK_DATA),
        seek::HOLE => Some(libc::SEEK_HOLE),
        _ => None,
    }
}

pub fn lock_type_to_host(linux: i16) -> Option<i16> {
    match linux {
        fcntl::RDLCK => Some(libc::F_RDLCK),
        fcntl::WRLCK => Some(libc::F_WRLCK),
        fcntl::UNLCK => Some(libc::F_UNLCK),
        _ => None,
    }
}

pub fn lock_type_from_host(host: i16) -> i16 {
    match host {
        libc::F_RDLCK => fcntl::RDLCK,
        libc::F_WRLCK => fcntl::WRLCK,
        _ => fcntl::UNLCK,
    }
}

pub fn map_flags_to_host(linux: i32) -> i32 {
    // cite: bionic mman-common.h#MAP_FIXED_NOREPLACE
    let mut host = 0;
    for (from, to) in [(mman::MAP_FIXED, libc::MAP_FIXED), (mman::MAP_ANONYMOUS, libc::MAP_ANON), (mman::MAP_NORESERVE, 0x40)] {
        if linux & from != 0 {
            host |= to;
        }
    }
    host | match linux & mman::MAP_TYPE {
        mman::MAP_SHARED | mman::MAP_SHARED_VALIDATE => libc::MAP_SHARED,
        _ => libc::MAP_PRIVATE,
    }
}

pub fn prot_to_host(linux: i32) -> i32 {
    (if linux & mman::PROT_READ != 0 { libc::PROT_READ } else { 0 }) | (if linux & mman::PROT_WRITE != 0 { libc::PROT_WRITE } else { 0 }) | (if linux & mman::PROT_EXEC != 0 { libc::PROT_EXEC } else { 0 })
}

pub fn at_remove_dir_to_host() -> i32 {
    libc::AT_REMOVEDIR
}

pub fn at_flags_known(flags: i32, allowed: i32) -> bool {
    flags & !allowed == 0
}

pub const AT_ALL_STAT: i32 = at::SYMLINK_NOFOLLOW | at::EMPTY_PATH;

pub fn message_flags_to_host(linux: i32) -> Option<i32> {
    // cite: bionic socket.h#MSG_NOSIGNAL, MSG_DONTWAIT
    let mut host = 0;
    let table = [
        (socket::MSG_OOB, libc::MSG_OOB),
        (socket::MSG_PEEK, libc::MSG_PEEK),
        (socket::MSG_DONTROUTE, libc::MSG_DONTROUTE),
        (socket::MSG_DONTWAIT, libc::MSG_DONTWAIT),
        (socket::MSG_EOR, libc::MSG_EOR),
        (socket::MSG_WAITALL, libc::MSG_WAITALL),
        (socket::MSG_NOSIGNAL, 0x80000),
    ];
    let mut remaining = linux;
    for (from, to) in table {
        if remaining & from != 0 {
            host |= to;
            remaining &= !from;
        }
    }
    remaining &= !(socket::MSG_MORE | socket::MSG_CMSG_CLOEXEC);
    (remaining == 0).then_some(host)
}

pub fn message_flags_from_host(host: i32) -> i32 {
    let mut linux = 0;
    for (from, to) in [(libc::MSG_EOR, socket::MSG_EOR), (libc::MSG_TRUNC, socket::MSG_TRUNC), (libc::MSG_CTRUNC, socket::MSG_CTRUNC), (libc::MSG_OOB, socket::MSG_OOB)] {
        if host & from != 0 {
            linux |= to;
        }
    }
    linux
}

pub fn family_to_host(linux: i32) -> Option<i32> {
    match linux {
        socket::AF_UNSPEC => Some(libc::AF_UNSPEC),
        socket::AF_UNIX => Some(libc::AF_UNIX),
        socket::AF_INET => Some(libc::AF_INET),
        socket::AF_INET6 => Some(libc::AF_INET6),
        _ => None,
    }
}

pub fn family_from_host(host: i32) -> i32 {
    match host {
        libc::AF_INET6 => socket::AF_INET6,
        other => other,
    }
}
