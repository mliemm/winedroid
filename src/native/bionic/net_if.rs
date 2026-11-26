use std::ffi::{CStr, CString};

use super::abi_types::{EFAULT, EINVAL, ENOMEM};
use super::export::in_libc;
use super::fs_result::{finish_int, missing};
use super::guest_mem::{read_value, write_bytes, write_value};
use super::linux_abi::{ioctl as requests, net_if as abi, socket as socket_abi};

const IFADDRS_SIZE: usize = 56;
const IF_NAMEINDEX_SIZE: usize = 16;

const IFF_MULTICAST_HOST: u32 = 0x8000;
const IFF_MULTICAST_LINUX: u32 = 0x1000;

const IFT_LOOP: u8 = 0x18;
const IFT_ETHER: u8 = 0x6;

const SIOCGIFADDR_HOST: libc::c_ulong = 0xc020_6921;
const SIOCGIFBRDADDR_HOST: libc::c_ulong = 0xc020_6923;
const SIOCGIFNETMASK_HOST: libc::c_ulong = 0xc020_6925;
const SIOCGIFFLAGS_HOST: libc::c_ulong = 0xc020_6911;
const SIOCGIFMTU_HOST: libc::c_ulong = 0xc020_6933;

fn error(code: i32) -> i64 {
    -i64::from(code)
}

fn linux_flags(host: u32) -> u32 {
    // cite: bionic if.h#IFF_UP, IFF_BROADCAST, IFF_LOOPBACK, IFF_RUNNING, IFF_MULTICAST
    let mut flags = host & 0x1ff;
    if host & IFF_MULTICAST_HOST != 0 {
        flags |= IFF_MULTICAST_LINUX;
    }
    flags
}

fn guest_sockaddr(storage: &libc::sockaddr_storage) -> (usize, u32) {
    match storage.ss_family as i32 {
        libc::AF_INET => {
            let address = unsafe { libc::malloc(socket_abi::SOCKADDR_SIZE) } as usize;
            let source =
                unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in) };
            write_value::<u16>(address, socket_abi::AF_INET as u16);
            write_value::<u16>(address + 2, source.sin_port);
            write_value::<u32>(address + 4, source.sin_addr.s_addr);
            (address, socket_abi::SOCKADDR_SIZE as u32)
        }
        libc::AF_INET6 => {
            let address = unsafe { libc::malloc(socket_abi::SOCKADDR_IN6_SIZE) } as usize;
            let source = unsafe {
                &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in6)
            };
            write_value::<u16>(address, socket_abi::AF_INET6 as u16);
            write_value::<u16>(address + 2, source.sin6_port);
            write_value::<u32>(address + 4, source.sin6_flowinfo);
            write_bytes(address + 8, &source.sin6_addr.s6_addr);
            write_value::<u32>(address + 24, source.sin6_scope_id);
            (address, socket_abi::SOCKADDR_IN6_SIZE as u32)
        }
        libc::AF_LINK => {
            let link =
                unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_dl) };
            let address = unsafe { libc::malloc(socket_abi::SOCKADDR_SIZE) } as usize;
            let kind = match link.sdl_type {
                IFT_LOOP => 772u16,
                IFT_ETHER => 1,
                _ => 1,
            };
            write_value::<u16>(address, kind);
            let mac = unsafe {
                std::slice::from_raw_parts(
                    link.sdl_data.as_ptr() as *const u8,
                    link.sdl_data.len().min(14),
                )
            };
            write_bytes(address + 2, mac);
            (address, socket_abi::SOCKADDR_SIZE as u32)
        }
        _ => (0, 0),
    }
}

fn guest_name(name: &CStr) -> usize {
    let bytes = name.to_bytes_with_nul();
    let pointer = unsafe { libc::malloc(bytes.len()) } as usize;
    write_bytes(pointer, bytes);
    pointer
}

fn get_ifaddrs(result: usize) -> i64 {
    // cite: bionic ifaddrs.cpp#getifaddrs
    if result == 0 {
        return error(EFAULT);
    }
    let mut host: *mut libc::ifaddrs = std::ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut host) } < 0 {
        return super::fs_result::from_host();
    }
    let mut head = 0usize;
    let mut tail = 0usize;
    let mut current = host;
    while !current.is_null() {
        let node = unsafe { &*current };
        let guest = unsafe { libc::malloc(IFADDRS_SIZE) } as usize;
        if guest == 0 {
            unsafe { libc::freeifaddrs(host) };
            return error(ENOMEM);
        }
        write_bytes(guest, &[0u8; IFADDRS_SIZE]);
        if !node.ifa_name.is_null() {
            write_value::<usize>(
                guest + 8,
                guest_name(unsafe { CStr::from_ptr(node.ifa_name) }),
            );
        }
        write_value::<u32>(guest + 16, linux_flags(node.ifa_flags));
        for (offset, source) in [
            (24usize, node.ifa_addr),
            (32, node.ifa_netmask),
            (40, node.ifa_dstaddr),
        ] {
            if !source.is_null() {
                let (address, _) =
                    guest_sockaddr(unsafe { &*(source as *const libc::sockaddr_storage) });
                write_value::<usize>(guest + offset, address);
            }
        }
        if tail == 0 {
            head = guest;
        } else {
            write_value::<usize>(tail, guest);
        }
        tail = guest;
        current = node.ifa_next;
    }
    unsafe { libc::freeifaddrs(host) };
    write_value::<usize>(result, head);
    0
}

fn free_ifaddrs(mut node: usize) {
    while node != 0 {
        let next = read_value::<usize>(node);
        let name = read_value::<usize>(node + 8);
        unsafe {
            if name != 0 {
                libc::free(name as *mut libc::c_void);
            }
            for offset in [24usize, 32, 40] {
                let address = read_value::<usize>(node + offset);
                if address != 0 {
                    libc::free(address as *mut libc::c_void);
                }
            }
            libc::free(node as *mut libc::c_void);
        }
        node = next;
    }
}

fn name_index(name: usize) -> i64 {
    if name == 0 {
        return error(EFAULT);
    }
    let text = unsafe { CStr::from_ptr(name as *const libc::c_char) };
    let index = unsafe { libc::if_nametoindex(text.as_ptr()) };
    if index == 0 {
        error((-super::fs_result::from_host()) as i32)
    } else {
        i64::from(index)
    }
}

fn index_name(index: u32, buffer: usize) -> i64 {
    // cite: bionic net_if.cpp#if_indextoname
    if buffer == 0 {
        return error(EFAULT);
    }
    let mut slot = [0i8; 32];
    let status = unsafe { libc::if_indextoname(index, slot.as_mut_ptr() as *mut libc::c_char) };
    if status.is_null() {
        return error((-super::fs_result::from_host()) as i32);
    }
    let text = unsafe { CStr::from_ptr(status) }.to_bytes_with_nul();
    write_bytes(buffer, text);
    buffer as i64
}

fn name_index_list() -> i64 {
    let host = unsafe { libc::if_nameindex() };
    if host.is_null() {
        return error(ENOMEM);
    }
    let mut count = 0usize;
    while unsafe { (*host.add(count)).if_index } != 0 {
        count += 1;
    }
    let list = unsafe { libc::malloc((count + 1) * IF_NAMEINDEX_SIZE) } as usize;
    if list == 0 {
        unsafe { libc::if_freenameindex(host) };
        return error(ENOMEM);
    }
    for index in 0..count {
        let entry = unsafe { &*host.add(index) };
        let name = unsafe { CStr::from_ptr(entry.if_name) }.to_bytes_with_nul();
        let copy = unsafe { libc::malloc(name.len()) } as usize;
        write_bytes(copy, name);
        write_value::<u32>(list + index * IF_NAMEINDEX_SIZE, entry.if_index);
        write_value::<usize>(list + index * IF_NAMEINDEX_SIZE + 8, copy);
    }
    write_bytes(list + count * IF_NAMEINDEX_SIZE, &[0u8; IF_NAMEINDEX_SIZE]);
    unsafe { libc::if_freenameindex(host) };
    list as i64
}

fn free_name_index_list(list: usize) {
    if list == 0 {
        return;
    }
    let mut cursor = list;
    loop {
        let index = read_value::<u32>(cursor);
        if index == 0 {
            break;
        }
        let name = read_value::<usize>(cursor + 8);
        unsafe {
            if name != 0 {
                libc::free(name as *mut libc::c_void);
            }
        }
        cursor += IF_NAMEINDEX_SIZE;
    }
    unsafe { libc::free(list as *mut libc::c_void) };
}

fn ifreq_host(name: &CStr) -> libc::ifreq {
    let mut request: libc::ifreq = unsafe { std::mem::zeroed() };
    let bytes = name.to_bytes();
    let length = bytes.len().min(abi::IFNAME_SIZE - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            request.ifr_name.as_mut_ptr() as *mut u8,
            length,
        )
    };
    request
}

fn ifreq_socket_name(pointer: usize) -> Result<CString, i32> {
    let bytes = unsafe { std::slice::from_raw_parts(pointer as *const u8, abi::IFNAME_SIZE) };
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    CString::new(&bytes[..end]).map_err(|_| EINVAL)
}

fn write_guest_sockaddr(target: usize, source: &libc::sockaddr) {
    let storage = unsafe { &*(source as *const libc::sockaddr as *const libc::sockaddr_storage) };
    let (address, _) = guest_sockaddr(storage);
    if address != 0 {
        write_bytes(target, unsafe {
            std::slice::from_raw_parts(address as *const u8, socket_abi::SOCKADDR_SIZE)
        });
        unsafe { libc::free(address as *mut libc::c_void) };
    }
}

fn host_name(index: u32) -> Option<CString> {
    let mut slot = [0i8; 32];
    let status = unsafe { libc::if_indextoname(index, slot.as_mut_ptr() as *mut libc::c_char) };
    (!status.is_null()).then(|| unsafe { CStr::from_ptr(status) }.to_owned())
}

fn link_address(name: &CStr) -> Option<(usize, u32)> {
    let mut host: *mut libc::ifaddrs = std::ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut host) } < 0 {
        return None;
    }
    let mut found = None;
    let mut current = host;
    while !current.is_null() {
        let node = unsafe { &*current };
        current = node.ifa_next;
        if node.ifa_addr.is_null() || unsafe { (*node.ifa_addr).sa_family } as i32 != libc::AF_LINK
        {
            continue;
        }
        if unsafe { CStr::from_ptr(node.ifa_name) } != name {
            continue;
        }
        found = Some(guest_sockaddr(unsafe {
            &*(node.ifa_addr as *const libc::sockaddr_storage)
        }));
        break;
    }
    unsafe { libc::freeifaddrs(host) };
    found.filter(|(address, _)| *address != 0)
}

pub fn request(fd: i32, request: u32, argument: usize) -> Option<i64> {
    // cite: bionic sockios.h#SIOCGIFADDR
    if argument == 0 {
        return Some(error(EFAULT));
    }
    if request == requests::SIOCGIFCONF {
        return Some(match interface_list(argument) {
            Ok(_) => 0,
            Err(errno) => error(errno),
        });
    }
    let name = match ifreq_socket_name(argument) {
        Ok(name) => name,
        Err(errno) => return Some(error(errno)),
    };
    let mut host = ifreq_host(&name);
    match request {
        abi::SIOCGIFADDR | abi::SIOCGIFBRDADDR | abi::SIOCGIFNETMASK => {
            let host_request = match request {
                abi::SIOCGIFADDR => SIOCGIFADDR_HOST,
                abi::SIOCGIFBRDADDR => SIOCGIFBRDADDR_HOST,
                _ => SIOCGIFNETMASK_HOST,
            };
            if unsafe { libc::ioctl(fd, host_request, &mut host) } < 0 {
                return Some(super::fs_result::from_host());
            }
            write_guest_sockaddr(argument + abi::IFNAME_SIZE, unsafe {
                &host.ifr_ifru.ifru_addr
            });
            Some(0)
        }
        abi::SIOCGIFFLAGS => {
            if unsafe { libc::ioctl(fd, SIOCGIFFLAGS_HOST, &mut host) } < 0 {
                return Some(super::fs_result::from_host());
            }
            let flags = unsafe { host.ifr_ifru.ifru_flags } as u16;
            write_value::<u16>(
                argument + abi::IFNAME_SIZE,
                linux_flags(u32::from(flags)) as u16,
            );
            Some(0)
        }
        abi::SIOCGIFMTU => {
            if unsafe { libc::ioctl(fd, SIOCGIFMTU_HOST, &mut host) } < 0 {
                return Some(super::fs_result::from_host());
            }
            write_value::<i32>(argument + abi::IFNAME_SIZE, unsafe {
                host.ifr_ifru.ifru_mtu
            });
            Some(0)
        }
        abi::SIOCGIFHWADDR => match link_address(&name) {
            Some((address, _)) => {
                write_bytes(argument + abi::IFNAME_SIZE, unsafe {
                    std::slice::from_raw_parts(address as *const u8, socket_abi::SOCKADDR_SIZE)
                });
                unsafe { libc::free(address as *mut libc::c_void) };
                Some(0)
            }
            None => Some(super::fs_result::from_host()),
        },
        abi::SIOCGIFINDEX => match unsafe { libc::if_nametoindex(name.as_ptr()) } {
            0 => Some(super::fs_result::from_host()),
            index => {
                write_value::<i32>(argument + abi::IFNAME_SIZE, index as i32);
                Some(0)
            }
        },
        abi::SIOCGIFNAME => {
            let index = read_value::<i32>(argument + abi::IFNAME_SIZE) as u32;
            match host_name(index) {
                Some(name) => {
                    let bytes = name.to_bytes();
                    let length = bytes.len().min(abi::IFNAME_SIZE - 1);
                    write_bytes(argument, &bytes[..length]);
                    write_value::<u8>(argument + length, 0);
                    Some(0)
                }
                None => Some(super::fs_result::from_host()),
            }
        }
        other => {
            missing(
                "ioctl",
                &format!("network request {other:#x} has no translation"),
            );
            None
        }
    }
}

fn interface_list(argument: usize) -> Result<i32, i32> {
    // cite: bionic if.h#ifconf
    let capacity = read_value::<i32>(argument);
    let destination = read_value::<usize>(argument + 8);
    if capacity <= 0 || destination == 0 {
        return Ok(0);
    }
    let mut host: *mut libc::ifaddrs = std::ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut host) } < 0 {
        return Err((-super::fs_result::from_host()) as i32);
    }
    let mut written = 0i32;
    let mut current = host;
    while !current.is_null() {
        let node = unsafe { &*current };
        current = node.ifa_next;
        if node.ifa_addr.is_null() || unsafe { (*node.ifa_addr).sa_family } as i32 != libc::AF_INET
        {
            continue;
        }
        if written + abi::IFREQ_SIZE as i32 > capacity {
            break;
        }
        let slot = destination + written as usize;
        write_bytes(slot, &[0u8; abi::IFREQ_SIZE]);
        let name = unsafe { CStr::from_ptr(node.ifa_name) }.to_bytes();
        let length = name.len().min(abi::IFNAME_SIZE - 1);
        write_bytes(slot, &name[..length]);
        write_guest_sockaddr(slot + abi::IFNAME_SIZE, unsafe { &*node.ifa_addr });
        written += abi::IFREQ_SIZE as i32;
    }
    unsafe { libc::freeifaddrs(host) };
    write_value::<i32>(argument, written);
    Ok(written)
}

pub fn register() {
    in_libc("getifaddrs", |context| {
        finish_int(context, get_ifaddrs(context.arg_ptr(0)))
    });
    in_libc("freeifaddrs", |context| free_ifaddrs(context.arg_ptr(0)));
    in_libc("if_nametoindex", |context| {
        finish_int(context, name_index(context.arg_ptr(0)))
    });
    in_libc("if_indextoname", |context| {
        finish_int(context, index_name(context.arg_u32(0), context.arg_ptr(1)))
    });
    in_libc("if_nameindex", |context| {
        let result = name_index_list();
        if result < 0 {
            finish_int(context, result);
        } else {
            context.ret_ptr(result as usize);
        }
    });
    in_libc("if_freenameindex", |context| {
        free_name_index_list(context.arg_ptr(0))
    });
}
