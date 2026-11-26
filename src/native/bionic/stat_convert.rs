use super::device_profile::{APP_GID, APP_UID};
use super::fd_table::SyntheticStat;
use super::linux_abi::stat as layout;

const OTHER_OWNER: u32 = 9999;

pub fn linux_device(darwin: i32) -> (u32, u32) {
    ((darwin as u32 >> 24) & 0xff, darwin as u32 & 0xff_ffff)
}

pub fn makedev(major: u32, minor: u32) -> u64 {
    // cite: bionic sysmacros.h#makedev
    let (major, minor) = (u64::from(major), u64::from(minor));
    (minor & 0xff) | ((major & 0xfff) << 8) | ((minor & !0xff) << 12) | ((major & !0xfff) << 32)
}

fn owner(host: u32, effective: u32, app: u32) -> u32 {
    if host == effective {
        app
    } else if host == 0 {
        0
    } else {
        OTHER_OWNER
    }
}

fn put(bytes: &mut [u8; layout::SIZE], offset: usize, value: &[u8]) {
    bytes[offset..offset + value.len()].copy_from_slice(value);
}

pub struct Fields {
    pub device: u64,
    pub inode: u64,
    pub mode: u32,
    pub links: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u64,
    pub size: i64,
    pub block_size: i32,
    pub blocks: i64,
    pub accessed: (i64, i64),
    pub modified: (i64, i64),
    pub changed: (i64, i64),
}

pub fn encode(fields: &Fields) -> [u8; layout::SIZE] {
    // cite: bionic stat.h#__STAT64_BODY
    let mut bytes = [0u8; layout::SIZE];
    put(&mut bytes, layout::ST_DEV, &fields.device.to_le_bytes());
    put(&mut bytes, layout::ST_INO, &fields.inode.to_le_bytes());
    put(&mut bytes, layout::ST_MODE, &fields.mode.to_le_bytes());
    put(&mut bytes, layout::ST_NLINK, &fields.links.to_le_bytes());
    put(&mut bytes, layout::ST_UID, &fields.uid.to_le_bytes());
    put(&mut bytes, layout::ST_GID, &fields.gid.to_le_bytes());
    put(&mut bytes, layout::ST_RDEV, &fields.rdev.to_le_bytes());
    put(&mut bytes, layout::ST_SIZE, &fields.size.to_le_bytes());
    put(
        &mut bytes,
        layout::ST_BLKSIZE,
        &fields.block_size.to_le_bytes(),
    );
    put(&mut bytes, layout::ST_BLOCKS, &fields.blocks.to_le_bytes());
    for (offset, (seconds, nanos)) in [
        (layout::ST_ATIME, fields.accessed),
        (layout::ST_MTIME, fields.modified),
        (layout::ST_CTIME, fields.changed),
    ] {
        put(&mut bytes, offset, &seconds.to_le_bytes());
        put(&mut bytes, offset + 8, &nanos.to_le_bytes());
    }
    bytes
}

pub fn from_host(host: &libc::stat, synthetic: Option<SyntheticStat>) -> [u8; layout::SIZE] {
    let (major, minor) = linux_device(host.st_dev);
    let mut fields = Fields {
        device: makedev(major, minor),
        inode: host.st_ino,
        mode: u32::from(host.st_mode),
        links: u32::from(host.st_nlink),
        uid: owner(host.st_uid, unsafe { libc::geteuid() }, APP_UID),
        gid: owner(host.st_gid, unsafe { libc::getegid() }, APP_GID),
        rdev: 0,
        size: host.st_size,
        block_size: host.st_blksize,
        blocks: host.st_blocks,
        accessed: (host.st_atime, host.st_atime_nsec),
        modified: (host.st_mtime, host.st_mtime_nsec),
        changed: (host.st_ctime, host.st_ctime_nsec),
    };
    if host.st_rdev != 0 {
        let (rdev_major, rdev_minor) = linux_device(host.st_rdev);
        fields.rdev = makedev(rdev_major, rdev_minor);
    }
    if let Some(synthetic) = synthetic {
        fields.mode = synthetic.mode;
        fields.device = makedev(synthetic.dev.0, synthetic.dev.1);
        fields.rdev = makedev(synthetic.rdev.0, synthetic.rdev.1);
        fields.links = 1;
        fields.uid = 0;
        fields.gid = 0;
        if synthetic.zero_size {
            fields.size = 0;
            fields.blocks = 0;
        }
    }
    encode(&fields)
}

pub fn virtual_node(
    mode: u32,
    owner_id: u32,
    inode: u64,
    device: (u32, u32),
    rdev: (u32, u32),
) -> [u8; layout::SIZE] {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    let time = (now, 0);
    encode(&Fields {
        device: makedev(device.0, device.1),
        inode,
        mode,
        links: if mode & 0o170000 == 0o040000 { 2 } else { 1 },
        uid: owner_id,
        gid: owner_id,
        rdev: makedev(rdev.0, rdev.1),
        size: if mode & 0o170000 == 0o040000 { 4096 } else { 0 },
        block_size: 4096,
        blocks: 0,
        accessed: time,
        modified: time,
        changed: time,
    })
}

pub fn path_hash(path: &str) -> u64 {
    path.bytes().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

pub const S_IFMT: u32 = 0o170000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFSOCK: u32 = 0o140000;
pub const S_IFIFO: u32 = 0o010000;

pub fn statfs_bytes(host: &libc::statfs) -> [u8; 120] {
    // cite: bionic sys_statfs.cpp#ST_VALID
    let mut bytes = [0u8; 120];
    let name = host
        .f_fstypename
        .iter()
        .take_while(|byte| **byte != 0)
        .map(|byte| *byte as u8)
        .collect::<Vec<u8>>();
    let kind: i64 = match name.as_slice() {
        b"apfs" | b"hfs" => 0xEF53,
        b"msdos" => 0x4d44,
        b"exfat" => 0x2011BAB0,
        b"nfs" => 0x6969,
        _ => 0x01021994,
    };
    let mut flags = 0i64;
    for (host_flag, linux_flag) in [
        (1u32, 1i64),
        (8, 2),
        (0x10, 4),
        (4, 8),
        (2, 0x10),
        (0x1000_0000, 0x400),
    ] {
        if host.f_flags & host_flag != 0 {
            flags |= linux_flag;
        }
    }
    let writes: [(usize, i64); 8] = [
        (0, kind),
        (8, i64::from(host.f_bsize)),
        (16, host.f_blocks as i64),
        (24, host.f_bfree as i64),
        (32, host.f_bavail as i64),
        (40, host.f_files as i64),
        (48, host.f_ffree as i64),
        (64, 255),
    ];
    for (offset, value) in writes {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    let fsid: [i32; 2] =
        unsafe { std::ptr::read_unaligned(&host.f_fsid as *const _ as *const [i32; 2]) };
    bytes[56..60].copy_from_slice(&fsid[0].to_le_bytes());
    bytes[60..64].copy_from_slice(&fsid[1].to_le_bytes());
    bytes[72..80].copy_from_slice(&i64::from(host.f_bsize).to_le_bytes());
    bytes[80..88].copy_from_slice(&flags.to_le_bytes());
    bytes
}
