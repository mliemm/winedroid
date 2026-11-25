pub mod open {
    // cite: bionic fcntl.h#O_CREAT, O_DIRECTORY, O_TMPFILE
    pub const ACCMODE: i32 = 3;
    pub const RDONLY: i32 = 0;
    pub const WRONLY: i32 = 1;
    pub const RDWR: i32 = 2;
    pub const CREAT: i32 = 0x40;
    pub const EXCL: i32 = 0x80;
    pub const NOCTTY: i32 = 0x100;
    pub const TRUNC: i32 = 0x200;
    pub const APPEND: i32 = 0x400;
    pub const NONBLOCK: i32 = 0x800;
    pub const DSYNC: i32 = 0x1000;
    pub const FASYNC: i32 = 0x2000;
    pub const DIRECTORY: i32 = 0x4000;
    pub const NOFOLLOW: i32 = 0x8000;
    pub const DIRECT: i32 = 0x10000;
    pub const LARGEFILE: i32 = 0x20000;
    pub const NOATIME: i32 = 0x40000;
    pub const CLOEXEC: i32 = 0x80000;
    pub const SYNC_BIT: i32 = 0x100000;
    pub const SYNC: i32 = SYNC_BIT | DSYNC;
    pub const PATH: i32 = 0x200000;
    pub const TMPFILE_BIT: i32 = 0x400000;
    pub const TMPFILE: i32 = TMPFILE_BIT | DIRECTORY;
}

pub mod fcntl {
    // cite: bionic fcntl.h#F_DUPFD, F_GETFD, F_SETFD, F_GETFL, F_SETFL, F_SETOWN, F_GETOWN, F_OFD_GETLK, FD_CLOEXEC, F_RDLCK, F_WRLCK, F_UNLCK, LOCK_SH, LOCK_EX, LOCK_NB, LOCK_UN

    // cite: bionic fcntl.h#F_ADD_SEALS, F_GET_SEALS, F_SEAL_SEAL, F_SEAL_SHRINK, F_SEAL_GROW, F_SEAL_WRITE, F_SEAL_FUTURE_WRITE
    pub const DUPFD: i32 = 0;
    pub const GETFD: i32 = 1;
    pub const SETFD: i32 = 2;
    pub const GETFL: i32 = 3;
    pub const SETFL: i32 = 4;
    pub const GETLK: i32 = 5;
    pub const SETLK: i32 = 6;
    pub const SETLKW: i32 = 7;
    pub const SETOWN: i32 = 8;
    pub const GETOWN: i32 = 9;
    pub const SETSIG: i32 = 10;
    pub const GETSIG: i32 = 11;
    pub const SETOWN_EX: i32 = 15;
    pub const GETOWN_EX: i32 = 16;
    pub const OFD_GETLK: i32 = 36;
    pub const OFD_SETLK: i32 = 37;
    pub const OFD_SETLKW: i32 = 38;
    pub const SETLEASE: i32 = 1024;
    pub const GETLEASE: i32 = 1025;
    pub const NOTIFY: i32 = 1026;
    pub const CANCELLK: i32 = 1029;
    pub const DUPFD_CLOEXEC: i32 = 1030;
    pub const SETPIPE_SZ: i32 = 1031;
    pub const GETPIPE_SZ: i32 = 1032;
    pub const ADD_SEALS: i32 = 1033;
    pub const GET_SEALS: i32 = 1034;
    pub const RDLCK: i16 = 0;
    pub const WRLCK: i16 = 1;
    pub const UNLCK: i16 = 2;
    pub const FD_CLOEXEC: i32 = 1;
    pub const LOCK_SH: i32 = 1;
    pub const LOCK_EX: i32 = 2;
    pub const LOCK_NB: i32 = 4;
    pub const LOCK_UN: i32 = 8;
    pub const SEAL_SEAL: u32 = 1;
    pub const SEAL_SHRINK: u32 = 2;
    pub const SEAL_GROW: u32 = 4;
    pub const SEAL_WRITE: u32 = 8;
    pub const SEAL_FUTURE_WRITE: u32 = 0x10;
}

pub mod at {
    // cite: bionic fcntl.h#AT_FDCWD, AT_SYMLINK_NOFOLLOW, AT_REMOVEDIR, AT_EACCESS, AT_SYMLINK_FOLLOW, AT_EMPTY_PATH
    pub const FDCWD: i32 = -100;
    pub const SYMLINK_NOFOLLOW: i32 = 0x100;
    pub const REMOVEDIR: i32 = 0x200;
    pub const EACCESS: i32 = 0x200;
    pub const SYMLINK_FOLLOW: i32 = 0x400;
    pub const EMPTY_PATH: i32 = 0x1000;
    // cite: bionic fs.h#RENAME_NOREPLACE
    pub const RENAME_NOREPLACE: u32 = 1;
    pub const RENAME_EXCHANGE: u32 = 2;
    pub const RENAME_WHITEOUT: u32 = 4;
}

pub mod seek {
    // cite: bionic seek_constants.h#SEEK_DATA
    pub const SET: i32 = 0;
    pub const CUR: i32 = 1;
    pub const END: i32 = 2;
    pub const DATA: i32 = 3;
    pub const HOLE: i32 = 4;
}

pub mod mman {
    // cite: bionic mman-common.h#MADV_DONTNEED, MAP_FIXED_NOREPLACE, MS_SYNC, PROT_GROWSDOWN
    pub const PROT_NONE: i32 = 0;
    pub const PROT_READ: i32 = 1;
    pub const PROT_WRITE: i32 = 2;
    pub const PROT_EXEC: i32 = 4;
    pub const PROT_SEM: i32 = 8;
    pub const PROT_BTI: i32 = 0x10;
    pub const PROT_MTE: i32 = 0x20;
    pub const PROT_GROWSDOWN: i32 = 0x01000000;
    pub const PROT_GROWSUP: i32 = 0x02000000;
    pub const MAP_SHARED: i32 = 1;
    pub const MAP_PRIVATE: i32 = 2;
    pub const MAP_SHARED_VALIDATE: i32 = 3;
    pub const MAP_TYPE: i32 = 0xf;
    pub const MAP_FIXED: i32 = 0x10;
    pub const MAP_ANONYMOUS: i32 = 0x20;
    pub const MAP_GROWSDOWN: i32 = 0x100;
    pub const MAP_DENYWRITE: i32 = 0x800;
    pub const MAP_EXECUTABLE: i32 = 0x1000;
    pub const MAP_LOCKED: i32 = 0x2000;
    pub const MAP_NORESERVE: i32 = 0x4000;
    pub const MAP_POPULATE: i32 = 0x8000;
    pub const MAP_NONBLOCK: i32 = 0x10000;
    pub const MAP_STACK: i32 = 0x20000;
    pub const MAP_HUGETLB: i32 = 0x40000;
    pub const MAP_SYNC: i32 = 0x80000;
    pub const MAP_FIXED_NOREPLACE: i32 = 0x100000;
    pub const MAP_UNINITIALIZED: i32 = 0x4000000;
    pub const MS_ASYNC: i32 = 1;
    pub const MS_INVALIDATE: i32 = 2;
    pub const MS_SYNC: i32 = 4;
    // cite: bionic mman.h#MREMAP_MAYMOVE
    pub const MREMAP_MAYMOVE: i32 = 1;
    pub const MREMAP_FIXED: i32 = 2;
    pub const MREMAP_DONTUNMAP: i32 = 4;
    pub const MCL_CURRENT: i32 = 1;
    pub const MCL_FUTURE: i32 = 2;
    pub const MCL_ONFAULT: i32 = 4;
    pub const MADV_NORMAL: i32 = 0;
    pub const MADV_RANDOM: i32 = 1;
    pub const MADV_SEQUENTIAL: i32 = 2;
    pub const MADV_WILLNEED: i32 = 3;
    pub const MADV_DONTNEED: i32 = 4;
    pub const MADV_FREE: i32 = 8;
    pub const MADV_REMOVE: i32 = 9;
    pub const MADV_DONTFORK: i32 = 10;
    pub const MADV_DOFORK: i32 = 11;
    pub const MADV_MERGEABLE: i32 = 12;
    pub const MADV_UNMERGEABLE: i32 = 13;
    pub const MADV_HUGEPAGE: i32 = 14;
    pub const MADV_NOHUGEPAGE: i32 = 15;
    pub const MADV_DONTDUMP: i32 = 16;
    pub const MADV_DODUMP: i32 = 17;
    pub const MADV_WIPEONFORK: i32 = 18;
    pub const MADV_KEEPONFORK: i32 = 19;
    pub const MADV_COLD: i32 = 20;
    pub const MADV_PAGEOUT: i32 = 21;
    pub const MADV_POPULATE_READ: i32 = 22;
    pub const MADV_POPULATE_WRITE: i32 = 23;
}

pub mod misc {
    // cite: bionic stat.h#UTIME_NOW, UTIME_OMIT
    pub const UTIME_NOW: i64 = (1 << 30) - 1;
    pub const UTIME_OMIT: i64 = (1 << 30) - 2;
    // cite: bionic falloc.h#FALLOC_FL_PUNCH_HOLE
    pub const FALLOC_FL_KEEP_SIZE: i32 = 1;
    pub const FALLOC_FL_PUNCH_HOLE: i32 = 2;
    pub const FALLOC_FL_ZERO_RANGE: i32 = 0x10;
    // cite: bionic memfd.h#MFD_ALLOW_SEALING
    pub const MFD_CLOEXEC: u32 = 1;
    pub const MFD_ALLOW_SEALING: u32 = 2;
    // cite: bionic close_range.h#CLOSE_RANGE_CLOEXEC
    pub const CLOSE_RANGE_UNSHARE: u32 = 2;
    pub const CLOSE_RANGE_CLOEXEC: u32 = 4;
    // cite: bionic dirent.h#__DIRENT64_BODY
    pub const DT_UNKNOWN: u8 = 0;
    pub const DT_FIFO: u8 = 1;
    pub const DT_CHR: u8 = 2;
    pub const DT_DIR: u8 = 4;
    pub const DT_BLK: u8 = 6;
    pub const DT_REG: u8 = 8;
    pub const DT_LNK: u8 = 10;
    pub const DT_SOCK: u8 = 12;
    // cite: bionic limits.h#PATH_MAX, NAME_MAX

    // cite: bionic limits.h#IOV_MAX
    pub const PATH_MAX: usize = 4096;
    pub const NAME_MAX: usize = 255;
    pub const IOV_MAX: i32 = 1024;
}

pub mod ioctl {
    // cite: bionic ioctls.h#FIONREAD, TIOCGWINSZ
    pub const TCGETS: u32 = 0x5401;
    pub const TIOCGWINSZ: u32 = 0x5413;
    pub const FIONREAD: u32 = 0x541B;
    pub const FIONBIO: u32 = 0x5421;
    pub const FIONCLEX: u32 = 0x5450;
    pub const FIOCLEX: u32 = 0x5451;
    pub const FIOASYNC: u32 = 0x5452;
    // cite: bionic sockios.h#SIOCGIFCONF
    pub const SIOCGIFCONF: u32 = 0x8912;
}

pub mod stat {
    // cite: bionic stat.h#__STAT64_BODY
    pub const SIZE: usize = 128;
    pub const ST_DEV: usize = 0;
    pub const ST_INO: usize = 8;
    pub const ST_MODE: usize = 16;
    pub const ST_NLINK: usize = 20;
    pub const ST_UID: usize = 24;
    pub const ST_GID: usize = 28;
    pub const ST_RDEV: usize = 32;
    pub const ST_SIZE: usize = 48;
    pub const ST_BLKSIZE: usize = 56;
    pub const ST_BLOCKS: usize = 64;
    pub const ST_ATIME: usize = 72;
    pub const ST_MTIME: usize = 88;
    pub const ST_CTIME: usize = 104;
    pub const DIRENT_SIZE: usize = 280;
    pub const DIRENT_RECLEN: usize = 16;
    pub const DIRENT_TYPE: usize = 18;
    pub const DIRENT_NAME: usize = 19;
}

pub mod poll {
    // cite: bionic poll.h#POLLIN, POLLRDHUP
    pub const IN: i16 = 1;
    pub const PRI: i16 = 2;
    pub const OUT: i16 = 4;
    pub const ERR: i16 = 8;
    pub const HUP: i16 = 0x10;
    pub const NVAL: i16 = 0x20;
    pub const RDNORM: i16 = 0x40;
    pub const RDBAND: i16 = 0x80;
    pub const WRNORM: i16 = 0x100;
    pub const WRBAND: i16 = 0x200;
    pub const RDHUP: i16 = 0x2000;
}

pub mod epoll {
    // cite: bionic eventpoll.h#EPOLLIN, EPOLLET, EPOLLONESHOT, EPOLLRDHUP, EPOLL_CTL_ADD
    pub const CTL_ADD: i32 = 1;
    pub const CTL_DEL: i32 = 2;
    pub const CTL_MOD: i32 = 3;
    pub const IN: u32 = 1;
    pub const PRI: u32 = 2;
    pub const OUT: u32 = 4;
    pub const ERR: u32 = 8;
    pub const HUP: u32 = 0x10;
    pub const RDNORM: u32 = 0x40;
    pub const RDBAND: u32 = 0x80;
    pub const WRNORM: u32 = 0x100;
    pub const WRBAND: u32 = 0x200;
    pub const RDHUP: u32 = 0x2000;
    pub const EXCLUSIVE: u32 = 1 << 28;
    pub const WAKEUP: u32 = 1 << 29;
    pub const ONESHOT: u32 = 1 << 30;
    pub const ET: u32 = 1 << 31;
    pub const CLOEXEC: i32 = 0x80000;
    // cite: bionic eventfd.h#EFD_SEMAPHORE
    pub const EFD_SEMAPHORE: i32 = 1;
    pub const EFD_NONBLOCK: i32 = 0x800;
    pub const EFD_CLOEXEC: i32 = 0x80000;
    // cite: bionic timerfd.h#TFD_TIMER_ABSTIME
    pub const TFD_TIMER_ABSTIME: i32 = 1;
    pub const TFD_TIMER_CANCEL_ON_SET: i32 = 2;
}

pub mod socket {
    // cite: bionic socket.h#SOCK_NONBLOCK, SOCK_CLOEXEC, SOMAXCONN, SHUT_RDWR
    pub const AF_UNSPEC: i32 = 0;
    pub const AF_UNIX: i32 = 1;
    pub const AF_INET: i32 = 2;
    pub const AF_INET6: i32 = 10;
    pub const AF_NETLINK: i32 = 16;
    pub const AF_PACKET: i32 = 17;
    pub const SHUT_RD: i32 = 0;
    pub const SHUT_WR: i32 = 1;
    pub const SHUT_RDWR: i32 = 2;
    pub const SOCKADDR_SIZE: usize = 16;
    pub const SOCKADDR_IN6_SIZE: usize = 28;
    pub const SOCKADDR_UN_SIZE: usize = 110;
    pub const SOCKADDR_STORAGE_SIZE: usize = 128;
    pub const SOCKADDR_UN_PATH: usize = 108;
    pub const SOCK_STREAM: i32 = 1;
    pub const SOCK_DGRAM: i32 = 2;
    pub const SOCK_RAW: i32 = 3;
    pub const SOCK_RDM: i32 = 4;
    pub const SOCK_SEQPACKET: i32 = 5;
    pub const SOCK_TYPE_MASK: i32 = 0xf;
    pub const SOCK_NONBLOCK: i32 = 0x800;
    pub const SOCK_CLOEXEC: i32 = 0x80000;
    pub const SOL_SOCKET: i32 = 1;
    pub const SOMAXCONN: i32 = 128;
    pub const MSG_OOB: i32 = 1;
    pub const MSG_PEEK: i32 = 2;
    pub const MSG_DONTROUTE: i32 = 4;
    pub const MSG_CTRUNC: i32 = 8;
    pub const MSG_TRUNC: i32 = 0x20;
    pub const MSG_DONTWAIT: i32 = 0x40;
    pub const MSG_EOR: i32 = 0x80;
    pub const MSG_WAITALL: i32 = 0x100;
    pub const MSG_NOSIGNAL: i32 = 0x4000;
    pub const MSG_MORE: i32 = 0x8000;
    pub const MSG_CMSG_CLOEXEC: i32 = 0x40000000;
    // cite: bionic socket.h#SO_REUSEADDR, SO_ERROR, SO_RCVTIMEO, SO_KEEPALIVE
    pub const SO_DEBUG: i32 = 1;
    pub const SO_REUSEADDR: i32 = 2;
    pub const SO_TYPE: i32 = 3;
    pub const SO_ERROR: i32 = 4;
    pub const SO_DONTROUTE: i32 = 5;
    pub const SO_BROADCAST: i32 = 6;
    pub const SO_SNDBUF: i32 = 7;
    pub const SO_RCVBUF: i32 = 8;
    pub const SO_KEEPALIVE: i32 = 9;
    pub const SO_OOBINLINE: i32 = 10;
    pub const SO_LINGER: i32 = 13;
    pub const SO_REUSEPORT: i32 = 15;
    pub const SO_RCVLOWAT: i32 = 18;
    pub const SO_SNDLOWAT: i32 = 19;
    pub const SO_RCVTIMEO: i32 = 20;
    pub const SO_SNDTIMEO: i32 = 21;
    pub const SO_ACCEPTCONN: i32 = 30;
    pub const SO_BINDTODEVICE: i32 = 25;
    pub const SO_PASSCRED: i32 = 16;
    pub const SO_PEERCRED: i32 = 17;
    pub const SO_PROTOCOL: i32 = 38;
    pub const SO_DOMAIN: i32 = 39;
    // cite: bionic in.h#IPPROTO_IP
    pub const IPPROTO_IP: i32 = 0;
    pub const IPPROTO_ICMP: i32 = 1;
    pub const IPPROTO_TCP: i32 = 6;
    pub const IPPROTO_UDP: i32 = 17;
    pub const IPPROTO_IPV6: i32 = 41;
    // cite: bionic tcp.h#TCP_NODELAY, TCP_KEEPIDLE
    pub const TCP_NODELAY: i32 = 1;
    pub const TCP_MAXSEG: i32 = 2;
    pub const TCP_KEEPIDLE: i32 = 4;
    pub const TCP_KEEPINTVL: i32 = 5;
    pub const TCP_KEEPCNT: i32 = 6;
    // cite: bionic in.h#IP_TOS, IP_MULTICAST_TTL
    pub const IP_TOS: i32 = 1;
    pub const IP_TTL: i32 = 2;
    pub const IP_MULTICAST_IF: i32 = 32;
    pub const IP_MULTICAST_TTL: i32 = 33;
    pub const IP_MULTICAST_LOOP: i32 = 34;
    // cite: bionic in6.h#IPV6_V6ONLY
    pub const IPV6_UNICAST_HOPS: i32 = 16;
    pub const IPV6_V6ONLY: i32 = 26;
}

pub mod netdb {
    // cite: bionic netdb.h#AI_PASSIVE, EAI_NONAME, EAI_NODATA
    pub const AI_PASSIVE: i32 = 1;
    pub const AI_CANONNAME: i32 = 2;
    pub const AI_NUMERICHOST: i32 = 4;
    pub const AI_NUMERICSERV: i32 = 8;
    pub const AI_ADDRCONFIG: i32 = 0x400;
    pub const AI_ALL: i32 = 0x100;
    pub const AI_V4MAPPED: i32 = 0x800;
    pub const AI_MASK: i32 = 0x1 | 0x2 | 0x4 | 0x8 | 0x400;
    pub const EAI_ADDRFAMILY: i32 = 1;
    pub const EAI_AGAIN: i32 = 2;
    pub const EAI_BADFLAGS: i32 = 3;
    pub const EAI_FAIL: i32 = 4;
    pub const EAI_FAMILY: i32 = 5;
    pub const EAI_MEMORY: i32 = 6;
    pub const EAI_NODATA: i32 = 7;
    pub const EAI_NONAME: i32 = 8;
    pub const EAI_SERVICE: i32 = 9;
    pub const EAI_SOCKTYPE: i32 = 10;
    pub const EAI_SYSTEM: i32 = 11;
    pub const EAI_BADHINTS: i32 = 12;
    pub const EAI_PROTOCOL: i32 = 13;
    pub const EAI_OVERFLOW: i32 = 14;
    pub const NI_NOFQDN: i32 = 1;
    pub const NI_NUMERICHOST: i32 = 2;
    pub const NI_NUMERICSERV: i32 = 8;
    pub const NI_NAMEREQD: i32 = 4;
    pub const NI_DGRAM: i32 = 0x10;
    pub const NI_MAXHOST: usize = 1025;
    pub const NI_MAXSERV: usize = 32;
    pub const AI_SIZE: usize = 48;
    pub const HOSTENT_SIZE: usize = 32;
}

pub mod net_if {
    // cite: bionic if.h#ifreq
    pub const IFREQ_SIZE: usize = 40;
    pub const IFNAME_SIZE: usize = 16;
    pub const SIOCGIFADDR: u32 = 0x8915;
    pub const SIOCGIFBRDADDR: u32 = 0x8919;
    pub const SIOCGIFNETMASK: u32 = 0x891b;
    pub const SIOCGIFFLAGS: u32 = 0x8913;
    pub const SIOCGIFHWADDR: u32 = 0x8927;
    pub const SIOCGIFINDEX: u32 = 0x8933;
    pub const SIOCGIFMTU: u32 = 0x8921;
    pub const SIOCGIFNAME: u32 = 0x8910;
    pub const IFF_UP: u16 = 0x1;
    pub const IFF_BROADCAST: u16 = 0x2;
    pub const IFF_LOOPBACK: u16 = 0x8;
    pub const IFF_RUNNING: u16 = 0x40;
    pub const IFF_MULTICAST: u16 = 0x1000;
}

pub mod stdio {
    // cite: bionic local.h#__SLBF, __SNBF, __SRD, __SWR, __SRW, __SEOF, __SERR, __SMBF, __SSTR, __SALC, __SIGN
    pub const SLBF: i32 = 0x0001;
    pub const SNBF: i32 = 0x0002;
    pub const SRD: i32 = 0x0004;
    pub const SWR: i32 = 0x0008;
    pub const SRW: i32 = 0x0010;
    pub const SEOF: i32 = 0x0020;
    pub const SERR: i32 = 0x0040;
    pub const SMBF: i32 = 0x0080;
    pub const SSTR: i32 = 0x0200;
    pub const SALC: i32 = 0x4000;
    pub const SIGN: i32 = 0x8000;
}
