use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::{Arc, Mutex, OnceLock};

use super::super::bridge::CallContext;
use super::abi_types::{EBADF, EFAULT, EINVAL};
use super::export::in_libc;
use super::fd_table::{self, FdEntry, FdKind};
use super::fs_result::{finish_int, from_host, missing};
use super::guest_mem::{read_value, write_bytes, write_value};
use super::guest_paths::{self, Access, HostRef, Resolved};
use super::linux_abi::{open, socket as abi};
use super::stat_convert::S_IFSOCK;
use super::translate::{
    family_from_host, family_to_host, message_flags_from_host, message_flags_to_host,
};

const EACCES: i32 = 13;
const EAFNOSUPPORT: i32 = 97;
const ECONNREFUSED: i32 = 111;
const ENOTCONN: i32 = 107;
const ENOPROTOOPT: i32 = 92;

const SOL_SOCKET_HOST: i32 = 0xffff;

pub struct SocketState {
    pub domain: i32,
    pub kind: i32,
    pub bound: Mutex<Option<String>>,
}

fn error(code: i32) -> i64 {
    -i64::from(code)
}

fn effective_errno() -> i64 {
    let failure = from_host();
    if failure == -i64::from(ECONNREFUSED) {
        return failure;
    }
    failure
}

fn suppress_sigpipe(fd: i32) {
    let enabled: i32 = 1;
    unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_NOSIGPIPE,
            &enabled as *const i32 as *const libc::c_void,
            4,
        )
    };
}

fn adopt_socket(fd: i32, domain: i32, kind: i32, flags: i32) -> Arc<FdEntry> {
    suppress_sigpipe(fd);
    let record = Arc::new(SocketState {
        domain,
        kind,
        bound: Mutex::new(None),
    });
    let entry = FdEntry::new(FdKind::Socket, "", open::RDWR | (flags & open::NONBLOCK))
        .with_synthetic(super::fd_table::SyntheticStat {
            mode: S_IFSOCK | 0o777,
            dev: (0, 0xe),
            rdev: (0, 0),
            zero_size: false,
        })
        .with_payload(record);
    fd_table::adopt(fd, entry)
}

const ENOTSOCK: i32 = 88;

fn state_of(fd: i32) -> Result<Arc<SocketState>, i64> {
    let entry = fd_table::get(fd).ok_or(error(EBADF))?;
    if entry.kind != FdKind::Socket {
        return Err(error(ENOTSOCK));
    }
    entry.payload_as::<SocketState>().ok_or(error(ENOTSOCK))
}

fn unix_bindings() -> &'static Mutex<HashMap<String, String>> {
    static BINDINGS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    BINDINGS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn host_path_of(reference: &HostRef) -> String {
    format!(
        "/dev/fd/{}/{}",
        reference.dir_fd(),
        reference.name().to_string_lossy()
    )
}

fn unix_address(path: &[u8], access: Access) -> Result<(libc::sockaddr_un, String, String), i32> {
    if path.first() == Some(&0) {
        missing("unix-socket", "abstract sockets have no Darwin equivalent");
        return Err(EINVAL);
    }
    let length = path
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(path.len());
    if length == 0 || length > abi::SOCKADDR_UN_PATH {
        return Err(EINVAL);
    }
    let guest = String::from_utf8_lossy(&path[..length]).into_owned();
    let reference = guest_paths::resolve_follow_absolute(&guest, access)?;
    let Resolved::Host(reference) = reference else {
        return Err(EACCES);
    };
    if reference.leaf.is_none() {
        return Err(EACCES);
    }
    let host = host_path_of(&reference);
    if host.len() >= 104 {
        return Err(super::abi_types::ENAMETOOLONG);
    }
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    address.sun_len = (2 + host.len()) as u8;
    address.sun_family = libc::AF_UNIX as u8;
    for (slot, byte) in address.sun_path.iter_mut().zip(host.bytes()) {
        *slot = byte as libc::c_char;
    }
    Ok((address, guest, host))
}

fn address_from_guest(
    pointer: usize,
    length: usize,
    access: Access,
) -> Result<(libc::sockaddr_storage, libc::socklen_t, Option<String>), i32> {
    if pointer == 0 {
        return Err(EFAULT);
    }
    if length < 2 {
        return Err(EINVAL);
    }
    let family = read_value::<u16>(pointer) as i32;
    let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    match family {
        abi::AF_UNSPEC => {
            storage.ss_family = 0;
            Ok((storage, 2, None))
        }
        abi::AF_UNIX => {
            let path = unsafe {
                std::slice::from_raw_parts(
                    (pointer + 2) as *const u8,
                    length.min(abi::SOCKADDR_UN_SIZE) - 2,
                )
            };
            if path.is_empty() {
                return Ok((storage, 2, None));
            }
            let (address, guest, _) = unix_address(path, access)?;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &address as *const _ as *const u8,
                    &mut storage as *mut _ as *mut u8,
                    std::mem::size_of::<libc::sockaddr_un>(),
                )
            };
            Ok((storage, address.sun_len as libc::socklen_t, Some(guest)))
        }
        abi::AF_INET => {
            if length < abi::SOCKADDR_SIZE {
                return Err(EINVAL);
            }
            let address = unsafe {
                &mut *(&mut storage as *mut libc::sockaddr_storage as *mut libc::sockaddr_in)
            };
            address.sin_len = abi::SOCKADDR_SIZE as u8;
            address.sin_family = libc::AF_INET as u8;
            address.sin_port = read_value::<u16>(pointer + 2);
            address.sin_addr.s_addr = read_value::<u32>(pointer + 4);
            Ok((storage, abi::SOCKADDR_SIZE as libc::socklen_t, None))
        }
        abi::AF_INET6 => {
            if length < abi::SOCKADDR_IN6_SIZE {
                return Err(EINVAL);
            }
            let address = unsafe {
                &mut *(&mut storage as *mut libc::sockaddr_storage as *mut libc::sockaddr_in6)
            };
            address.sin6_len = abi::SOCKADDR_IN6_SIZE as u8;
            address.sin6_family = libc::AF_INET6 as u8;
            address.sin6_port = read_value::<u16>(pointer + 2);
            address.sin6_flowinfo = read_value::<u32>(pointer + 4);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    (pointer + 8) as *const u8,
                    address.sin6_addr.s6_addr.as_mut_ptr(),
                    16,
                )
            };
            address.sin6_scope_id = read_value::<u32>(pointer + 24);
            Ok((storage, abi::SOCKADDR_IN6_SIZE as libc::socklen_t, None))
        }
        _ => Err(EAFNOSUPPORT),
    }
}

fn guest_path_of_host(host: &str) -> Option<String> {
    unix_bindings()
        .lock()
        .expect("unix bindings")
        .get(host)
        .cloned()
}

fn address_to_guest(
    storage: &libc::sockaddr_storage,
    length: libc::socklen_t,
    pointer: usize,
    capacity: usize,
) -> Result<libc::socklen_t, i32> {
    if pointer == 0 {
        return Ok(0);
    }
    let family = storage.ss_family as i32;
    let needed = match family {
        libc::AF_UNIX => abi::SOCKADDR_UN_SIZE,
        libc::AF_INET => abi::SOCKADDR_SIZE,
        libc::AF_INET6 => abi::SOCKADDR_IN6_SIZE,
        _ => length as usize,
    };
    let written = needed.min(capacity);
    let mut buffer = vec![0u8; needed];
    match family {
        libc::AF_UNIX => {
            let address =
                unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_un) };
            let host = unsafe { CStr::from_ptr(address.sun_path.as_ptr()) }
                .to_string_lossy()
                .into_owned();
            let guest = guest_path_of_host(&host).unwrap_or_else(|| host.clone());
            let bytes = guest.as_bytes();
            let copied = bytes.len().min(abi::SOCKADDR_UN_PATH);
            buffer[..2].copy_from_slice(&(abi::AF_UNIX as u16).to_le_bytes());
            buffer[2..2 + copied].copy_from_slice(&bytes[..copied]);
        }
        libc::AF_INET => {
            let address =
                unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in) };
            buffer[..2].copy_from_slice(&(abi::AF_INET as u16).to_le_bytes());
            buffer[2..4].copy_from_slice(&address.sin_port.to_ne_bytes());
            buffer[4..8].copy_from_slice(&address.sin_addr.s_addr.to_le_bytes());
        }
        libc::AF_INET6 => {
            let address = unsafe {
                &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in6)
            };
            buffer[..2].copy_from_slice(&(abi::AF_INET6 as u16).to_le_bytes());
            buffer[2..4].copy_from_slice(&address.sin6_port.to_ne_bytes());
            buffer[4..8].copy_from_slice(&address.sin6_flowinfo.to_le_bytes());
            buffer[8..24].copy_from_slice(&address.sin6_addr.s6_addr);
            buffer[24..28].copy_from_slice(&address.sin6_scope_id.to_le_bytes());
        }
        other => {
            buffer[..2].copy_from_slice(&(family_from_host(other) as u16).to_le_bytes());
        }
    }
    write_bytes(pointer, &buffer[..written]);
    Ok(written as libc::socklen_t)
}

pub fn create(domain: i32, kind: i32, protocol: i32) -> i64 {
    // cite: bionic socket.h#socket
    let flags = kind & (abi::SOCK_NONBLOCK | abi::SOCK_CLOEXEC);
    let host_kind = kind & !(abi::SOCK_NONBLOCK | abi::SOCK_CLOEXEC);
    if host_kind < abi::SOCK_STREAM || host_kind > abi::SOCK_SEQPACKET {
        return error(EINVAL);
    }
    let Some(host_domain) = family_to_host(domain) else {
        missing(
            "socket",
            &format!("address family {domain} has no host equivalent"),
        );
        return error(EAFNOSUPPORT);
    };
    let fd = unsafe { libc::socket(host_domain, host_kind, protocol) };
    if fd < 0 {
        return effective_errno();
    }
    if flags & abi::SOCK_CLOEXEC != 0 {
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    if flags & abi::SOCK_NONBLOCK != 0 {
        unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };
    }
    adopt_socket(fd, domain, host_kind, flags);
    i64::from(fd)
}

fn bind_at(fd: i32, pointer: usize, length: usize) -> i64 {
    let state = match state_of(fd) {
        Ok(state) => state,
        Err(failure) => return failure,
    };
    let (storage, host_length, guest) = match address_from_guest(pointer, length, Access::Write) {
        Ok(parts) => parts,
        Err(errno) => return error(errno),
    };
    let status = unsafe {
        libc::bind(
            fd,
            &storage as *const _ as *const libc::sockaddr,
            host_length,
        )
    };
    if status < 0 {
        return effective_errno();
    }
    if let Some(guest) = guest {
        *state.bound.lock().expect("socket state") = Some(guest.clone());
        if let Ok((_, _, host)) = unix_address(guest.as_bytes(), Access::Write) {
            unix_bindings()
                .lock()
                .expect("unix bindings")
                .insert(host, guest);
        }
    }
    0
}

fn connect_at(fd: i32, pointer: usize, length: usize) -> i64 {
    // cite: bionic socket.h#connect
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    let (storage, host_length, _) = match address_from_guest(pointer, length, Access::Write) {
        Ok(parts) => parts,
        Err(errno) => return error(errno),
    };
    if unsafe {
        libc::connect(
            fd,
            &storage as *const _ as *const libc::sockaddr,
            host_length,
        )
    } < 0
    {
        return effective_errno();
    }
    0
}

fn listen_fd(fd: i32, backlog: i32) -> i64 {
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if unsafe { libc::listen(fd, backlog) } < 0 {
        from_host()
    } else {
        0
    }
}

fn accept_common(fd: i32, pointer: usize, length_pointer: usize, flags: i32) -> i64 {
    // cite: bionic socket.h#accept4
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if flags & !(abi::SOCK_NONBLOCK | abi::SOCK_CLOEXEC) != 0 {
        return error(EINVAL);
    }
    let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    let mut host_length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    if pointer != 0 && length_pointer == 0 {
        return error(EFAULT);
    }
    let accepted = unsafe {
        libc::accept(
            fd,
            &mut storage as *mut _ as *mut libc::sockaddr,
            &mut host_length,
        )
    };
    if accepted < 0 {
        return effective_errno();
    }
    if flags & abi::SOCK_CLOEXEC != 0 {
        unsafe { libc::fcntl(accepted, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    if flags & abi::SOCK_NONBLOCK != 0 {
        unsafe { libc::fcntl(accepted, libc::F_SETFL, libc::O_NONBLOCK) };
    }
    let (domain, kind) = fd_table::get(fd)
        .and_then(|entry| entry.payload_as::<SocketState>())
        .map(|state| (state.domain, state.kind))
        .unwrap_or((abi::AF_UNSPEC, abi::SOCK_STREAM));
    adopt_socket(accepted, domain, kind, flags);
    if pointer != 0 {
        let capacity = read_value::<u32>(length_pointer) as usize;
        match address_to_guest(&storage, host_length, pointer, capacity) {
            Ok(written) => write_value::<u32>(length_pointer, written as u32),
            Err(errno) => return error(errno),
        }
    }
    i64::from(accepted)
}

fn send_flags(linux: i32) -> Result<i32, i32> {
    message_flags_to_host(linux).ok_or(EINVAL)
}

fn receive(fd: i32, buffer: usize, length: usize, flags: i32) -> i64 {
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if buffer == 0 && length > 0 {
        return error(EFAULT);
    }
    let host = match send_flags(flags) {
        Ok(flags) => flags,
        Err(errno) => return error(errno),
    };
    let received = unsafe { libc::recv(fd, buffer as *mut libc::c_void, length, host) };
    if received < 0 {
        effective_errno()
    } else {
        received as i64
    }
}

fn transmit(fd: i32, buffer: usize, length: usize, flags: i32) -> i64 {
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if buffer == 0 && length > 0 {
        return error(EFAULT);
    }
    let host = match send_flags(flags) {
        Ok(flags) => flags,
        Err(errno) => return error(errno),
    };
    let sent = unsafe { libc::send(fd, buffer as *const libc::c_void, length, host) };
    if sent < 0 {
        effective_errno()
    } else {
        sent as i64
    }
}

fn receive_from(
    fd: i32,
    buffer: usize,
    length: usize,
    flags: i32,
    address: usize,
    address_length: usize,
) -> i64 {
    let host = match send_flags(flags) {
        Ok(flags) => flags,
        Err(errno) => return error(errno),
    };
    let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    let mut host_length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    let received = unsafe {
        libc::recvfrom(
            fd,
            buffer as *mut libc::c_void,
            length,
            host,
            if address == 0 {
                std::ptr::null_mut()
            } else {
                &mut storage as *mut _ as *mut libc::sockaddr
            },
            if address == 0 {
                std::ptr::null_mut()
            } else {
                &mut host_length
            },
        )
    };
    if received < 0 {
        return effective_errno();
    }
    if address != 0 && address_length != 0 {
        let capacity = read_value::<u32>(address_length) as usize;
        match address_to_guest(&storage, host_length, address, capacity) {
            Ok(written) => write_value::<u32>(address_length, written as u32),
            Err(errno) => return error(errno),
        }
    }
    received as i64
}

fn transmit_to(
    fd: i32,
    buffer: usize,
    length: usize,
    flags: i32,
    address: usize,
    address_length: usize,
) -> i64 {
    let host = match send_flags(flags) {
        Ok(flags) => flags,
        Err(errno) => return error(errno),
    };
    if address == 0 {
        return transmit(fd, buffer, length, flags);
    }
    let (storage, host_length, _) = match address_from_guest(address, address_length, Access::Write)
    {
        Ok(parts) => parts,
        Err(errno) => return error(errno),
    };
    let sent = unsafe {
        libc::sendto(
            fd,
            buffer as *const libc::c_void,
            length,
            host,
            &storage as *const _ as *const libc::sockaddr,
            host_length,
        )
    };
    if sent < 0 {
        effective_errno()
    } else {
        sent as i64
    }
}

fn cmsg_align(length: usize) -> usize {
    (length + 7) & !7
}

fn cmsg_align_host(length: usize) -> usize {
    (length + 3) & !3
}

fn control_to_host(guest: usize, length: usize) -> Result<(Vec<u8>, bool), i32> {
    let mut output = Vec::with_capacity(length);
    let mut truncated = false;
    let mut offset = 0usize;
    while offset + 16 <= length {
        let header = guest + offset;
        let claimed = read_value::<u64>(header) as usize;
        let level = read_value::<i32>(header + 8);
        let kind = read_value::<i32>(header + 12);
        if claimed < 16 || offset + claimed > length {
            return Err(EINVAL);
        }
        let data = header + 16;
        let data_length = claimed - 16;
        if level == abi::SOL_SOCKET && kind == SCM_RIGHTS {
            let host_offset = output.len();
            output.resize(host_offset + cmsg_align_host(12 + data_length), 0);
            let host_len = 12 + data_length;
            output[host_offset..host_offset + 4].copy_from_slice(&(host_len as u32).to_le_bytes());
            output[host_offset + 4..host_offset + 8]
                .copy_from_slice(&SCM_RIGHTS_HOST.to_le_bytes());
            output[host_offset + 8..host_offset + 12].copy_from_slice(&kind.to_le_bytes());
            let count = data_length / 4;
            for index in 0..count {
                let fd = read_value::<i32>(data + index * 4);
                output[host_offset + 12 + index * 4..host_offset + 16 + index * 4]
                    .copy_from_slice(&fd.to_le_bytes());
            }
        } else {
            missing(
                "sendmsg",
                &format!("control message level {level} type {kind} has no host equivalent"),
            );
            truncated = true;
        }
        offset += cmsg_align(claimed);
    }
    Ok((output, truncated))
}

const SCM_RIGHTS: i32 = 1;

const SCM_RIGHTS_HOST: i32 = 1;

fn control_to_guest(
    host: usize,
    length: usize,
    guest: usize,
    capacity: usize,
    cloexec: bool,
) -> (usize, bool, i32) {
    let mut output = Vec::new();
    let mut truncated = false;
    let mut received_fds = 0;
    let mut offset = 0usize;
    while offset + 12 <= length {
        let header = host + offset;
        let claimed = read_value::<u32>(header) as usize;
        let level = read_value::<i32>(header + 4);
        let kind = read_value::<i32>(header + 8);
        if claimed < 12 || offset + claimed > length {
            break;
        }
        if level == SOL_SOCKET_HOST && kind == SCM_RIGHTS {
            let data_length = claimed - 12;
            let count = data_length / std::mem::size_of::<libc::c_int>();
            let aligned = (16 + count * 4 + 7) & !7;
            let start = output.len();
            output.resize(start + aligned, 0);
            output[start..start + 8].copy_from_slice(&((16 + count * 4) as u64).to_le_bytes());
            output[start + 8..start + 12].copy_from_slice(&abi::SOL_SOCKET.to_le_bytes());
            output[start + 12..start + 16].copy_from_slice(&SCM_RIGHTS.to_le_bytes());
            for index in 0..count {
                let fd = read_value::<i32>(header + 12 + index * 4);
                if fd >= 0 {
                    if cloexec {
                        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
                    }
                    fd_table::adopt(
                        fd,
                        FdEntry::new(FdKind::Host, "", open::RDWR).with_synthetic(
                            super::fd_table::SyntheticStat {
                                mode: S_IFSOCK | 0o777,
                                dev: (0, 0xe),
                                rdev: (0, 0),
                                zero_size: false,
                            },
                        ),
                    );
                    received_fds += 1;
                }
                output[start + 16 + index * 4..start + 20 + index * 4]
                    .copy_from_slice(&fd.to_le_bytes());
            }
        } else if claimed > 12 {
            missing(
                "recvmsg",
                &format!("control message level {level} type {kind} has no host equivalent"),
            );
            truncated = true;
        }
        offset += cmsg_align_host(claimed);
    }
    let written = output.len().min(capacity);
    write_bytes(guest, &output[..written]);
    (written, truncated, received_fds)
}

fn send_message(fd: i32, header_pointer: usize, flags: i32) -> i64 {
    // cite: bionic socket.h#sendmsg
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if header_pointer == 0 {
        return error(EFAULT);
    }
    let name = read_value::<usize>(header_pointer);
    let name_length = read_value::<u32>(header_pointer + 8) as usize;
    let iov_pointer = read_value::<usize>(header_pointer + 16);
    let iov_count = read_value::<u64>(header_pointer + 24) as usize;
    let control = read_value::<usize>(header_pointer + 32);
    let control_length = read_value::<u64>(header_pointer + 40) as usize;
    if iov_count > 0 && iov_pointer == 0 {
        return error(EFAULT);
    }
    let Some(host_flags) = message_flags_to_host(flags) else {
        return error(EINVAL);
    };
    let mut vectors = Vec::with_capacity(iov_count);
    for index in 0..iov_count {
        vectors.push(libc::iovec {
            iov_base: read_value::<usize>(iov_pointer + index * 16) as *mut libc::c_void,
            iov_len: read_value::<usize>(iov_pointer + index * 16 + 8),
        });
    }
    let (mut control_bytes, truncated) = match control_length {
        0 => (Vec::new(), false),
        _ => match control_to_host(control, control_length) {
            Ok(parts) => parts,
            Err(errno) => return error(errno),
        },
    };
    let (storage, host_name_length, _) = match name {
        0 => (
            unsafe { std::mem::zeroed::<libc::sockaddr_storage>() },
            0,
            None,
        ),
        _ => match address_from_guest(name, name_length, Access::Write) {
            Ok(parts) => parts,
            Err(errno) => return error(errno),
        },
    };
    let header = libc::msghdr {
        msg_name: if name == 0 {
            std::ptr::null_mut()
        } else {
            &storage as *const _ as *mut libc::c_void
        },
        msg_namelen: host_name_length,
        msg_iov: vectors.as_mut_ptr(),
        msg_iovlen: vectors.len() as libc::c_int,
        msg_control: if control_bytes.is_empty() {
            std::ptr::null_mut()
        } else {
            control_bytes.as_mut_ptr() as *mut libc::c_void
        },
        msg_controllen: if truncated {
            0
        } else {
            control_bytes.len() as libc::socklen_t
        },
        msg_flags: 0,
    };
    let sent = unsafe { libc::sendmsg(fd, &header, host_flags) };
    if sent < 0 {
        return effective_errno();
    }
    sent as i64
}

fn receive_message(fd: i32, header_pointer: usize, flags: i32) -> i64 {
    // cite: bionic recvmsg.cpp#recvmsg
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if header_pointer == 0 {
        return error(EFAULT);
    }
    let name = read_value::<usize>(header_pointer);
    let name_capacity = read_value::<u32>(header_pointer + 8) as usize;
    let iov_pointer = read_value::<usize>(header_pointer + 16);
    let iov_count = read_value::<u64>(header_pointer + 24) as usize;
    let control = read_value::<usize>(header_pointer + 32);
    let control_capacity = read_value::<u64>(header_pointer + 40) as usize;
    if iov_count > 0 && iov_pointer == 0 {
        return error(EFAULT);
    }
    let Some(host_flags) = message_flags_to_host(flags) else {
        return error(EINVAL);
    };
    let mut vectors = Vec::with_capacity(iov_count);
    for index in 0..iov_count {
        vectors.push(libc::iovec {
            iov_base: read_value::<usize>(iov_pointer + index * 16) as *mut libc::c_void,
            iov_len: read_value::<usize>(iov_pointer + index * 16 + 8),
        });
    }
    let mut control_bytes = vec![0u8; control_capacity.max(1)];
    let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    let mut header = libc::msghdr {
        msg_name: if name == 0 {
            std::ptr::null_mut()
        } else {
            &mut storage as *mut _ as *mut libc::c_void
        },
        msg_namelen: name_capacity as libc::socklen_t,
        msg_iov: vectors.as_mut_ptr(),
        msg_iovlen: vectors.len() as libc::c_int,
        msg_control: if control_capacity == 0 {
            std::ptr::null_mut()
        } else {
            control_bytes.as_mut_ptr() as *mut libc::c_void
        },
        msg_controllen: control_capacity as libc::socklen_t,
        msg_flags: 0,
    };
    let received = unsafe { libc::recvmsg(fd, &mut header, host_flags) };
    if received < 0 {
        return effective_errno();
    }
    if name != 0 {
        match address_to_guest(&storage, header.msg_namelen, name, name_capacity) {
            Ok(written) => write_value::<u32>(header_pointer + 8, written as u32),
            Err(errno) => return error(errno),
        }
    }
    if control_capacity != 0 && header.msg_controllen > 0 {
        let (written, truncated, _) = control_to_guest(
            header.msg_control as usize,
            header.msg_controllen as usize,
            control,
            control_capacity,
            flags & abi::MSG_CMSG_CLOEXEC != 0,
        );
        write_value::<u64>(header_pointer + 40, written as u64);
        if truncated {
            header.msg_flags |= libc::MSG_CTRUNC;
        }
    } else if control_capacity != 0 {
        write_value::<u64>(header_pointer + 40, 0);
    }
    write_value::<i32>(
        header_pointer + 48,
        message_flags_from_host(header.msg_flags),
    );
    received as i64
}

fn name_at(fd: i32, pointer: usize, length_pointer: usize, peer: bool) -> i64 {
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if pointer == 0 || length_pointer == 0 {
        return error(EFAULT);
    }
    if !peer {
        if let Ok(state) = state_of(fd) {
            if let Some(guest) = state.bound.lock().expect("socket state").clone() {
                let capacity = read_value::<u32>(length_pointer) as usize;
                let mut buffer = vec![0u8; abi::SOCKADDR_UN_SIZE];
                let bytes = guest.as_bytes();
                let copied = bytes.len().min(abi::SOCKADDR_UN_PATH);
                buffer[..2].copy_from_slice(&(abi::AF_UNIX as u16).to_le_bytes());
                buffer[2..2 + copied].copy_from_slice(&bytes[..copied]);
                write_bytes(pointer, &buffer[..capacity.min(buffer.len())]);
                write_value::<u32>(length_pointer, (2 + copied) as u32);
                return 0;
            }
        }
    }
    let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    let mut host_length = std::mem::size_of::<libc::sockaddr_storage>() as libc::socklen_t;
    let status = unsafe {
        if peer {
            libc::getpeername(
                fd,
                &mut storage as *mut _ as *mut libc::sockaddr,
                &mut host_length,
            )
        } else {
            libc::getsockname(
                fd,
                &mut storage as *mut _ as *mut libc::sockaddr,
                &mut host_length,
            )
        }
    };
    if status < 0 {
        let failure = from_host();
        return if failure == -i64::from(ENOTCONN) {
            error(ENOTCONN)
        } else {
            failure
        };
    }
    let capacity = read_value::<u32>(length_pointer) as usize;
    match address_to_guest(&storage, host_length, pointer, capacity) {
        Ok(written) => {
            write_value::<u32>(length_pointer, written);
            0
        }
        Err(errno) => error(errno),
    }
}

fn shutdown_fd(fd: i32, how: i32) -> i64 {
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if !matches!(how, abi::SHUT_RD | abi::SHUT_WR | abi::SHUT_RDWR) {
        return error(EINVAL);
    }
    if unsafe { libc::shutdown(fd, how) } < 0 {
        from_host()
    } else {
        0
    }
}

fn protocol_to_host(level: i32) -> Option<i32> {
    match level {
        abi::SOL_SOCKET => Some(SOL_SOCKET_HOST),
        abi::IPPROTO_IP => Some(0),
        abi::IPPROTO_TCP => Some(6),
        abi::IPPROTO_IPV6 => Some(41),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum OptionKind {
    Int,
    Linger,
    Timeval,
    MulticastInterface,
}

fn socket_option(level: i32, name: i32) -> Option<(i32, OptionKind)> {
    // cite: bionic socket.h#SO_REUSEADDR, SO_LINGER, SO_RCVTIMEO
    match level {
        abi::SOL_SOCKET => match name {
            abi::SO_DEBUG => Some((0x0001, OptionKind::Int)),
            abi::SO_REUSEADDR => Some((0x0004, OptionKind::Int)),
            abi::SO_TYPE => Some((0x1008, OptionKind::Int)),
            abi::SO_ERROR => Some((0x1007, OptionKind::Int)),
            abi::SO_DONTROUTE => Some((0x0010, OptionKind::Int)),
            abi::SO_BROADCAST => Some((0x0020, OptionKind::Int)),
            abi::SO_SNDBUF => Some((0x1001, OptionKind::Int)),
            abi::SO_RCVBUF => Some((0x1002, OptionKind::Int)),
            abi::SO_KEEPALIVE => Some((0x0008, OptionKind::Int)),
            abi::SO_OOBINLINE => Some((0x0100, OptionKind::Int)),
            abi::SO_LINGER => Some((0x0080, OptionKind::Linger)),
            abi::SO_REUSEPORT => Some((0x0200, OptionKind::Int)),
            abi::SO_RCVLOWAT => Some((0x1004, OptionKind::Int)),
            abi::SO_SNDLOWAT => Some((0x1003, OptionKind::Int)),
            abi::SO_RCVTIMEO => Some((0x1006, OptionKind::Timeval)),
            abi::SO_SNDTIMEO => Some((0x1005, OptionKind::Timeval)),
            abi::SO_ACCEPTCONN => Some((0x0002, OptionKind::Int)),
            _ => None,
        },
        abi::IPPROTO_TCP => match name {
            abi::TCP_NODELAY => Some((0x01, OptionKind::Int)),
            abi::TCP_MAXSEG => Some((0x02, OptionKind::Int)),
            abi::TCP_KEEPIDLE => Some((0x10, OptionKind::Int)),
            abi::TCP_KEEPINTVL => Some((0x101, OptionKind::Int)),
            abi::TCP_KEEPCNT => Some((0x102, OptionKind::Int)),
            _ => None,
        },
        abi::IPPROTO_IP => match name {
            abi::IP_TOS => Some((3, OptionKind::Int)),
            abi::IP_TTL => Some((4, OptionKind::Int)),
            abi::IP_MULTICAST_IF => Some((9, OptionKind::MulticastInterface)),
            abi::IP_MULTICAST_TTL => Some((10, OptionKind::Int)),
            abi::IP_MULTICAST_LOOP => Some((11, OptionKind::Int)),
            _ => None,
        },
        abi::IPPROTO_IPV6 => match name {
            abi::IPV6_UNICAST_HOPS => Some((4, OptionKind::Int)),
            abi::IPV6_V6ONLY => Some((27, OptionKind::Int)),
            _ => None,
        },
        _ => None,
    }
}

fn host_timeval(guest: usize) -> libc::timeval {
    let seconds = read_value::<i64>(guest);
    let micros = read_value::<i64>(guest + 8).clamp(0, 999_999);
    libc::timeval {
        tv_sec: seconds as libc::time_t,
        tv_usec: micros as libc::suseconds_t,
    }
}

fn guest_timeval(host: &libc::timeval, guest: usize) {
    write_value::<i64>(guest, host.tv_sec as i64);
    write_value::<i64>(guest + 8, host.tv_usec as i64);
}

fn set_option(fd: i32, level: i32, name: i32, value: usize, length: usize) -> i64 {
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if value == 0 {
        return error(EFAULT);
    }
    let Some(host_level) = protocol_to_host(level) else {
        return error(ENOPROTOOPT);
    };
    let Some((host_name, kind)) = socket_option(level, name) else {
        missing(
            "setsockopt",
            &format!("option level {level} name {name} has no host equivalent"),
        );
        return error(ENOPROTOOPT);
    };
    let minimum = match kind {
        OptionKind::Int => 4,
        OptionKind::Linger => 8,
        OptionKind::Timeval => 16,
        OptionKind::MulticastInterface => 4,
    };
    if length < minimum {
        return error(EINVAL);
    }
    let (pointer, host_length) = match kind {
        OptionKind::Int => (value as *const libc::c_void, 4usize),
        OptionKind::Linger => (value as *const libc::c_void, 8usize),
        OptionKind::Timeval => {
            let time = host_timeval(value);
            return finish_option(unsafe {
                libc::setsockopt(
                    fd,
                    host_level,
                    host_name,
                    &time as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::timeval>() as libc::socklen_t,
                )
            });
        }
        OptionKind::MulticastInterface => (value as *const libc::c_void, 4usize),
    };
    finish_option(unsafe {
        libc::setsockopt(
            fd,
            host_level,
            host_name,
            pointer,
            host_length as libc::socklen_t,
        )
    })
}

fn finish_option(status: i32) -> i64 {
    if status < 0 {
        from_host()
    } else {
        0
    }
}

fn get_option(fd: i32, level: i32, name: i32, value: usize, length_pointer: usize) -> i64 {
    if state_of(fd).is_err() {
        return error(ENOTSOCK);
    }
    if value == 0 || length_pointer == 0 {
        return error(EFAULT);
    }
    let capacity = read_value::<u32>(length_pointer) as usize;
    if level == abi::SOL_SOCKET {
        if let Ok(state) = state_of(fd) {
            match name {
                abi::SO_DOMAIN => {
                    write_value::<i32>(value, state.domain);
                    write_value::<u32>(length_pointer, 4);
                    return 0;
                }
                abi::SO_PROTOCOL => {
                    write_value::<i32>(value, 0);
                    write_value::<u32>(length_pointer, 4);
                    return 0;
                }
                _ => {}
            }
        }
    }
    let Some(host_level) = protocol_to_host(level) else {
        return error(ENOPROTOOPT);
    };
    let Some((host_name, kind)) = socket_option(level, name) else {
        missing(
            "getsockopt",
            &format!("option level {level} name {name} has no host equivalent"),
        );
        return error(ENOPROTOOPT);
    };
    let mut int_value: libc::c_int = 0;
    let mut linger: libc::linger = unsafe { std::mem::zeroed() };
    let mut time: libc::timeval = unsafe { std::mem::zeroed() };
    let (pointer, host_length, guest_length) = match kind {
        OptionKind::Int | OptionKind::MulticastInterface => (
            &mut int_value as *mut _ as *mut libc::c_void,
            4usize,
            4usize,
        ),
        OptionKind::Linger => (&mut linger as *mut _ as *mut libc::c_void, 8usize, 8usize),
        OptionKind::Timeval => (
            &mut time as *mut _ as *mut libc::c_void,
            std::mem::size_of::<libc::timeval>(),
            16usize,
        ),
    };
    let mut length = host_length as libc::socklen_t;
    if unsafe { libc::getsockopt(fd, host_level, host_name, pointer, &mut length) } < 0 {
        return from_host();
    }
    match kind {
        OptionKind::Int | OptionKind::MulticastInterface => write_value::<i32>(value, int_value),
        OptionKind::Linger => {
            write_value::<i32>(value, linger.l_onoff);
            write_value::<i32>(value + 4, linger.l_linger);
        }
        OptionKind::Timeval => guest_timeval(&time, value),
    }
    write_value::<u32>(length_pointer, guest_length.min(capacity) as u32);
    0
}

fn socket_pair(domain: i32, kind: i32, protocol: i32, pointer: usize) -> i64 {
    // cite: bionic socketpair.cpp#socketpair
    if pointer == 0 {
        return error(EFAULT);
    }
    let Some(host_domain) = family_to_host(domain) else {
        return error(EAFNOSUPPORT);
    };
    let flags = kind & (abi::SOCK_NONBLOCK | abi::SOCK_CLOEXEC);
    let host_kind = kind & !(abi::SOCK_NONBLOCK | abi::SOCK_CLOEXEC);
    let mut pair = [0i32; 2];
    if unsafe { libc::socketpair(host_domain, host_kind, protocol, pair.as_mut_ptr()) } < 0 {
        return from_host();
    }
    for fd in pair {
        if flags & abi::SOCK_CLOEXEC != 0 {
            unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
        }
        if flags & abi::SOCK_NONBLOCK != 0 {
            unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };
        }
        adopt_socket(fd, domain, host_kind, flags);
    }
    write_value::<i32>(pointer, pair[0]);
    write_value::<i32>(pointer + 4, pair[1]);
    0
}

fn handle(context: &mut CallContext, result: i64) {
    finish_int(context, result);
}

pub fn send_fd(fd: i32, buffer: usize, length: usize, flags: i32) -> i64 {
    transmit(fd, buffer, length, flags)
}

pub fn recv_fd(fd: i32, buffer: usize, length: usize, flags: i32) -> i64 {
    receive(fd, buffer, length, flags)
}

pub fn register() {
    in_libc("socket", |context| {
        let result = create(context.arg_i32(0), context.arg_i32(1), context.arg_i32(2));
        handle(context, result);
    });
    in_libc("bind", |context| {
        handle(
            context,
            bind_at(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_u32(2) as usize,
            ),
        )
    });
    in_libc("connect", |context| {
        handle(
            context,
            connect_at(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_u32(2) as usize,
            ),
        )
    });
    in_libc("listen", |context| {
        handle(context, listen_fd(context.arg_i32(0), context.arg_i32(1)))
    });
    in_libc("accept", |context| {
        handle(
            context,
            accept_common(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_ptr(2),
                0,
            ),
        )
    });
    in_libc("accept4", |context| {
        handle(
            context,
            accept_common(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_ptr(2),
                context.arg_i32(3),
            ),
        )
    });
    in_libc("send", |context| {
        handle(
            context,
            transmit(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_u64(2) as usize,
                context.arg_i32(3),
            ),
        )
    });
    in_libc("recv", |context| {
        handle(
            context,
            receive(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_u64(2) as usize,
                context.arg_i32(3),
            ),
        )
    });
    in_libc("sendto", |context| {
        handle(
            context,
            transmit_to(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_u64(2) as usize,
                context.arg_i32(3),
                context.arg_ptr(4),
                context.arg_u32(5) as usize,
            ),
        )
    });
    in_libc("recvfrom", |context| {
        handle(
            context,
            receive_from(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_u64(2) as usize,
                context.arg_i32(3),
                context.arg_ptr(4),
                context.arg_ptr(5),
            ),
        )
    });
    in_libc("sendmsg", |context| {
        handle(
            context,
            send_message(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2)),
        )
    });
    in_libc("recvmsg", |context| {
        handle(
            context,
            receive_message(context.arg_i32(0), context.arg_ptr(1), context.arg_i32(2)),
        )
    });
    in_libc("shutdown", |context| {
        handle(context, shutdown_fd(context.arg_i32(0), context.arg_i32(1)))
    });
    in_libc("socketpair", |context| {
        handle(
            context,
            socket_pair(
                context.arg_i32(0),
                context.arg_i32(1),
                context.arg_i32(2),
                context.arg_ptr(3),
            ),
        )
    });
    in_libc("setsockopt", |context| {
        handle(
            context,
            set_option(
                context.arg_i32(0),
                context.arg_i32(1),
                context.arg_i32(2),
                context.arg_ptr(3),
                context.arg_u32(4) as usize,
            ),
        )
    });
    in_libc("getsockopt", |context| {
        handle(
            context,
            get_option(
                context.arg_i32(0),
                context.arg_i32(1),
                context.arg_i32(2),
                context.arg_ptr(3),
                context.arg_ptr(4),
            ),
        )
    });
    in_libc("getsockname", |context| {
        handle(
            context,
            name_at(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_ptr(2),
                false,
            ),
        )
    });
    in_libc("getpeername", |context| {
        handle(
            context,
            name_at(
                context.arg_i32(0),
                context.arg_ptr(1),
                context.arg_ptr(2),
                true,
            ),
        )
    });
}
