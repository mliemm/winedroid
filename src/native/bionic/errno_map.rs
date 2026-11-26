pub struct ErrnoRow {
    pub name: &'static str,
    pub linux: i32,
    pub darwin: i32,
}

const fn row(name: &'static str, linux: i32, darwin: i32) -> ErrnoRow {
    ErrnoRow {
        name,
        linux,
        darwin,
    }
}

// cite: bionic errno-base.h#EPERM, EDOM, ERANGE
// cite: bionic errno.h#EDEADLK, EOWNERDEAD, EHWPOISON
pub const ROWS: &[ErrnoRow] = &[
    row("EPERM", 1, 1),
    row("ENOENT", 2, 2),
    row("ESRCH", 3, 3),
    row("EINTR", 4, 4),
    row("EIO", 5, 5),
    row("ENXIO", 6, 6),
    row("E2BIG", 7, 7),
    row("ENOEXEC", 8, 8),
    row("EBADF", 9, 9),
    row("ECHILD", 10, 10),
    row("EAGAIN", 11, 35),
    row("ENOMEM", 12, 12),
    row("EACCES", 13, 13),
    row("EFAULT", 14, 14),
    row("ENOTBLK", 15, 15),
    row("EBUSY", 16, 16),
    row("EEXIST", 17, 17),
    row("EXDEV", 18, 18),
    row("ENODEV", 19, 19),
    row("ENOTDIR", 20, 20),
    row("EISDIR", 21, 21),
    row("EINVAL", 22, 22),
    row("ENFILE", 23, 23),
    row("EMFILE", 24, 24),
    row("ENOTTY", 25, 25),
    row("ETXTBSY", 26, 26),
    row("EFBIG", 27, 27),
    row("ENOSPC", 28, 28),
    row("ESPIPE", 29, 29),
    row("EROFS", 30, 30),
    row("EMLINK", 31, 31),
    row("EPIPE", 32, 32),
    row("EDOM", 33, 33),
    row("ERANGE", 34, 34),
    row("EDEADLK", 35, 11),
    row("ENAMETOOLONG", 36, 63),
    row("ENOLCK", 37, 77),
    row("ENOSYS", 38, 78),
    row("ENOTEMPTY", 39, 66),
    row("ELOOP", 40, 62),
    row("ENOMSG", 42, 91),
    row("EIDRM", 43, 90),
    row("ENOSTR", 60, 99),
    row("ENODATA", 61, 96),
    row("ETIME", 62, 101),
    row("ENOSR", 63, 98),
    row("EREMOTE", 66, 71),
    row("ENOLINK", 67, 97),
    row("EPROTO", 71, 100),
    row("EMULTIHOP", 72, 95),
    row("EBADMSG", 74, 94),
    row("EOVERFLOW", 75, 84),
    row("EILSEQ", 84, 92),
    row("EUSERS", 87, 68),
    row("ENOTSOCK", 88, 38),
    row("EDESTADDRREQ", 89, 39),
    row("EMSGSIZE", 90, 40),
    row("EPROTOTYPE", 91, 41),
    row("ENOPROTOOPT", 92, 42),
    row("EPROTONOSUPPORT", 93, 43),
    row("ESOCKTNOSUPPORT", 94, 44),
    row("EOPNOTSUPP", 95, 102),
    row("EPFNOSUPPORT", 96, 46),
    row("EAFNOSUPPORT", 97, 47),
    row("EADDRINUSE", 98, 48),
    row("EADDRNOTAVAIL", 99, 49),
    row("ENETDOWN", 100, 50),
    row("ENETUNREACH", 101, 51),
    row("ENETRESET", 102, 52),
    row("ECONNABORTED", 103, 53),
    row("ECONNRESET", 104, 54),
    row("ENOBUFS", 105, 55),
    row("EISCONN", 106, 56),
    row("ENOTCONN", 107, 57),
    row("ESHUTDOWN", 108, 58),
    row("ETOOMANYREFS", 109, 59),
    row("ETIMEDOUT", 110, 60),
    row("ECONNREFUSED", 111, 61),
    row("EHOSTDOWN", 112, 64),
    row("EHOSTUNREACH", 113, 65),
    row("EALREADY", 114, 37),
    row("EINPROGRESS", 115, 36),
    row("ESTALE", 116, 70),
    row("EDQUOT", 122, 69),
    row("ECANCELED", 125, 89),
    row("EOWNERDEAD", 130, 105),
    row("ENOTRECOVERABLE", 131, 104),
];

pub const DARWIN_ALIASES: &[(&str, i32, i32)] = &[("ENOTSUP", 45, 95), ("ENOATTR", 93, 61)];

pub const LINUX_ERRNO_MAX: i32 = 133;

pub fn linux_from_darwin(darwin: i32) -> Option<i32> {
    if darwin == 0 {
        return Some(0);
    }
    ROWS.iter()
        .find(|row| row.darwin == darwin)
        .map(|row| row.linux)
        .or_else(|| {
            DARWIN_ALIASES
                .iter()
                .find(|alias| alias.1 == darwin)
                .map(|alias| alias.2)
        })
}

pub fn darwin_from_linux(linux: i32) -> Option<i32> {
    if linux == 0 {
        return Some(0);
    }
    ROWS.iter()
        .find(|row| row.linux == linux)
        .map(|row| row.darwin)
}

pub fn is_linux_errno(code: i32) -> bool {
    (0..=LINUX_ERRNO_MAX).contains(&code)
}
