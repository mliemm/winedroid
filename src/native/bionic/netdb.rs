use std::cell::{Cell, RefCell};
use std::ffi::CString;

use super::super::bridge::CallContext;
use super::abi_types::EFAULT;
use super::export::in_libc;
use super::fs_result::{finish_int, missing};
use super::guest_mem::{cstr_bytes, optional_cstr, read_value, write_bytes, write_value};
use super::linux_abi::{netdb as codes, socket as abi};

const EAI_SYSTEM_LOCAL: i32 = 11;

const HOSTENT_SIZE: usize = 32;
const ADDINFO_SIZE: usize = 48;
const NAME_NOT_FOUND: i32 = 1;
const TRY_AGAIN: i32 = 2;
const NO_RECOVERY: i32 = 3;
const NO_DATA: i32 = 4;

fn error(code: i32) -> i64 {
    -i64::from(code)
}

fn host_ai_flags(linux: i32) -> i32 {
    let mut host = 0;
    for (from, to) in [(codes::AI_PASSIVE, libc::AI_PASSIVE), (codes::AI_CANONNAME, libc::AI_CANONNAME), (codes::AI_NUMERICHOST, libc::AI_NUMERICHOST), (codes::AI_ADDRCONFIG, libc::AI_ADDRCONFIG), (codes::AI_V4MAPPED, libc::AI_V4MAPPED), (codes::AI_ALL, libc::AI_ALL)] {
        if linux & from != 0 {
            host |= to;
        }
    }
    if linux & codes::AI_NUMERICSERV != 0 {
        host |= libc::AI_NUMERICSERV;
    }
    host
}

thread_local! {
    static H_ERRNO: Cell<i32> = const { Cell::new(0) };
    static HOSTENT: RefCell<Option<HostentStorage>> = const { RefCell::new(None) };
}

fn set_h_errno(value: i32) {
    H_ERRNO.with(|slot| slot.set(value));
}

fn h_errno_address() -> usize {
    H_ERRNO.with(|slot| slot.as_ptr() as usize)
}

fn name_at(pointer: usize) -> Option<Result<CString, i32>> {
    if pointer == 0 {
        return None;
    }
    Some(CString::new(cstr_bytes(pointer)).map_err(|_| codes::EAI_NONAME))
}

fn allocate(bytes: usize) -> usize {
    (unsafe { libc::malloc(bytes) }) as usize
}

fn guest_sockaddr(storage: &libc::sockaddr_storage, length: libc::socklen_t) -> (usize, u32) {
    match storage.ss_family as i32 {
        libc::AF_INET => {
            let address = allocate(abi::SOCKADDR_SIZE);
            let source = unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in) };
            write_value::<u16>(address, abi::AF_INET as u16);
            write_value::<u16>(address + 2, source.sin_port);
            write_value::<u32>(address + 4, source.sin_addr.s_addr);
            (address, abi::SOCKADDR_SIZE as u32)
        }
        libc::AF_INET6 => {
            let address = allocate(abi::SOCKADDR_IN6_SIZE);
            let source = unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in6) };
            write_value::<u16>(address, abi::AF_INET6 as u16);
            write_value::<u16>(address + 2, source.sin6_port);
            write_value::<u32>(address + 4, source.sin6_flowinfo);
            write_bytes(address + 8, &source.sin6_addr.s6_addr);
            write_value::<u32>(address + 24, source.sin6_scope_id);
            (address, abi::SOCKADDR_IN6_SIZE as u32)
        }
        _ => {
            let address = allocate(length.max(2) as usize);
            write_value::<u16>(address, abi::AF_UNSPEC as u16);
            (address, length.max(2) as u32)
        }
    }
}

fn guest_addrinfo(host: &libc::addrinfo) -> usize {
    let node = allocate(ADDINFO_SIZE);
    write_bytes(node, &[0u8; ADDINFO_SIZE]);
    write_value::<i32>(node, host.ai_flags);
    write_value::<i32>(node + 4, super::translate::family_from_host(host.ai_family));
    write_value::<i32>(node + 8, host.ai_socktype);
    write_value::<i32>(node + 12, host.ai_protocol);
    let (address, length) = if host.ai_addr.is_null() {
        (0, 0)
    } else {
        let storage = unsafe { &*(host.ai_addr as *const libc::sockaddr_storage) };
        guest_sockaddr(storage, host.ai_addrlen)
    };
    write_value::<u32>(node + 16, length);
    if !host.ai_canonname.is_null() {
        let text = unsafe { std::ffi::CStr::from_ptr(host.ai_canonname) }.to_bytes_with_nul();
        let copy = allocate(text.len());
        write_bytes(copy, text);
        write_value::<usize>(node + 24, copy);
    }
    write_value::<usize>(node + 32, address);
    node
}

fn resolve(node: Option<CString>, service: Option<CString>, hints: usize) -> Result<usize, i32> {
    // cite: bionic getaddrinfo.c#getaddrinfo
    let mut host_hints: libc::addrinfo = unsafe { std::mem::zeroed() };
    let hints_pointer = if hints == 0 {
        std::ptr::null()
    } else {
        let flags = read_value::<i32>(hints);
        if flags & !codes::AI_MASK != 0 {
            return Err(codes::EAI_BADFLAGS);
        }
        host_hints.ai_flags = host_ai_flags(flags);
        host_hints.ai_family = match super::translate::family_to_host(read_value::<i32>(hints + 4)) {
            Some(family) => family,
            None => return Err(codes::EAI_FAMILY),
        };
        host_hints.ai_socktype = read_value::<i32>(hints + 8);
        host_hints.ai_protocol = read_value::<i32>(hints + 12);
        &host_hints
    };
    let mut result: *mut libc::addrinfo = std::ptr::null_mut();
    let status = unsafe { libc::getaddrinfo(node.as_ref().map_or(std::ptr::null(), |value| value.as_ptr()), service.as_ref().map_or(std::ptr::null(), |value| value.as_ptr()), hints_pointer, &mut result) };
    if status != 0 {
        return Err(status);
    }
    let mut head = 0usize;
    let mut tail = 0usize;
    let mut current = result;
    while !current.is_null() {
        let guest = guest_addrinfo(unsafe { &*current });
        if tail == 0 {
            head = guest;
        } else {
            write_value::<usize>(tail + 40, guest);
        }
        tail = guest;
        current = unsafe { (*current).ai_next };
    }
    unsafe { libc::freeaddrinfo(result) };
    Ok(head)
}

fn free_list(mut node: usize) {
    while node != 0 {
        let next = read_value::<usize>(node + 40);
        let address = read_value::<usize>(node + 32);
        let canonname = read_value::<usize>(node + 24);
        unsafe {
            if address != 0 {
                libc::free(address as *mut libc::c_void);
            }
            if canonname != 0 {
                libc::free(canonname as *mut libc::c_void);
            }
            libc::free(node as *mut libc::c_void);
        }
        node = next;
    }
}

fn storage_of(pointer: usize, length: usize) -> Result<(libc::sockaddr_storage, libc::socklen_t), i32> {
    if pointer == 0 || length < 2 {
        return Err(codes::EAI_FAMILY);
    }
    let family = read_value::<u16>(pointer) as i32;
    let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
    match family {
        abi::AF_INET => {
            if length < abi::SOCKADDR_SIZE {
                return Err(codes::EAI_FAMILY);
            }
            let address = unsafe { &mut *(&mut storage as *mut libc::sockaddr_storage as *mut libc::sockaddr_in) };
            address.sin_len = abi::SOCKADDR_SIZE as u8;
            address.sin_family = libc::AF_INET as u8;
            address.sin_port = read_value::<u16>(pointer + 2);
            address.sin_addr.s_addr = read_value::<u32>(pointer + 4);
            Ok((storage, abi::SOCKADDR_SIZE as libc::socklen_t))
        }
        abi::AF_INET6 => {
            if length < abi::SOCKADDR_IN6_SIZE {
                return Err(codes::EAI_FAMILY);
            }
            let address = unsafe { &mut *(&mut storage as *mut libc::sockaddr_storage as *mut libc::sockaddr_in6) };
            address.sin6_len = abi::SOCKADDR_IN6_SIZE as u8;
            address.sin6_family = libc::AF_INET6 as u8;
            address.sin6_port = read_value::<u16>(pointer + 2);
            address.sin6_flowinfo = read_value::<u32>(pointer + 4);
            unsafe { std::ptr::copy_nonoverlapping((pointer + 8) as *const u8, address.sin6_addr.s6_addr.as_mut_ptr(), 16) };
            address.sin6_scope_id = read_value::<u32>(pointer + 24);
            Ok((storage, abi::SOCKADDR_IN6_SIZE as libc::socklen_t))
        }
        _ => Err(codes::EAI_FAMILY),
    }
}

fn copy_out(text: &[u8], pointer: usize, capacity: usize) -> Result<(), i32> {
    if pointer == 0 || capacity == 0 {
        return Ok(());
    }
    if text.len() >= capacity {
        return Err(codes::EAI_OVERFLOW);
    }
    write_bytes(pointer, text);
    write_value::<u8>(pointer + text.len(), 0);
    Ok(())
}

fn name_info(pointer: usize, length: usize, host: usize, host_capacity: usize, service: usize, service_capacity: usize, flags: i32) -> i32 {
    // cite: bionic getnameinfo.c#getnameinfo
    if flags & !(codes::NI_NOFQDN | codes::NI_NUMERICHOST | codes::NI_NAMEREQD | codes::NI_NUMERICSERV | codes::NI_DGRAM) != 0 {
        return codes::EAI_BADFLAGS;
    }
    let (storage, host_length) = match storage_of(pointer, length) {
        Ok(parts) => parts,
        Err(code) => return code,
    };
    let mut host_buffer = vec![0u8; host_capacity.clamp(1, codes::NI_MAXHOST)];
    let mut service_buffer = vec![0u8; service_capacity.clamp(1, codes::NI_MAXSERV)];
    let status = unsafe {
        libc::getnameinfo(
            &storage as *const _ as *const libc::sockaddr,
            host_length,
            if host == 0 { std::ptr::null_mut() } else { host_buffer.as_mut_ptr() as *mut libc::c_char },
            if host == 0 { 0 } else { host_buffer.len() as libc::socklen_t },
            if service == 0 { std::ptr::null_mut() } else { service_buffer.as_mut_ptr() as *mut libc::c_char },
            if service == 0 { 0 } else { service_buffer.len() as libc::socklen_t },
            flags,
        )
    };
    if status != 0 {
        return status;
    }
    if host != 0 {
        let text = unsafe { std::ffi::CStr::from_ptr(host_buffer.as_ptr() as *const libc::c_char) }.to_bytes();
        if let Err(code) = copy_out(text, host, host_capacity) {
            return code;
        }
    }
    if service != 0 {
        let text = unsafe { std::ffi::CStr::from_ptr(service_buffer.as_ptr() as *const libc::c_char) }.to_bytes();
        if let Err(code) = copy_out(text, service, service_capacity) {
            return code;
        }
    }
    0
}

struct HostentStorage {
    _bytes: Box<[u8]>,
}

fn hostent_from(name: &[u8], family: i32, address: &[u8]) -> usize {
    let name_length = name.len() + 1;
    let total = HOSTENT_SIZE + name_length + 8 + address.len() + 8;
    let mut bytes = vec![0u8; total].into_boxed_slice();
    let base = bytes.as_ptr() as usize;
    bytes[..name.len()].copy_from_slice(name);
    let name_pointer = base;
    let alias_pointer = base + name_length;
    let address_pointer = base + name_length + 8;
    let list_pointer = base + name_length + 8 + address.len();
    unsafe {
        std::ptr::write_unaligned((base + 0) as *mut usize, name_pointer);
        std::ptr::write_unaligned((base + 8) as *mut usize, alias_pointer);
        std::ptr::write_unaligned((base + 16) as *mut i32, family);
        std::ptr::write_unaligned((base + 20) as *mut i32, address.len() as i32);
        std::ptr::write_unaligned((base + 24) as *mut usize, list_pointer);
        std::ptr::write_unaligned(address_pointer as *mut usize, 0);
        std::ptr::write_unaligned(list_pointer as *mut usize, address_pointer);
        std::ptr::write_unaligned((list_pointer + 8) as *mut usize, 0);
    }
    bytes[address_pointer - base..address_pointer - base + address.len()].copy_from_slice(address);
    HOSTENT.with(|slot| {
        *slot.borrow_mut() = Some(HostentStorage { _bytes: bytes });
    });
    base
}

fn lookup_host(name: Option<CString>, family: i32) -> Result<usize, i32> {
    let Some(name) = name else {
        set_h_errno(NO_RECOVERY);
        return Err(codes::EAI_NONAME);
    };
    let mut hints: libc::addrinfo = unsafe { std::mem::zeroed() };
    hints.ai_family = family;
    hints.ai_socktype = abi::SOCK_STREAM;
    let mut result: *mut libc::addrinfo = std::ptr::null_mut();
    let status = unsafe { libc::getaddrinfo(name.as_ptr(), std::ptr::null(), &hints, &mut result) };
    if status != 0 {
        set_h_errno(match status {
            codes::EAI_AGAIN => TRY_AGAIN,
            codes::EAI_NONAME | codes::EAI_NODATA | codes::EAI_ADDRFAMILY => if status == codes::EAI_NONAME { NAME_NOT_FOUND } else { NO_DATA },
            _ => NO_RECOVERY,
        });
        return Err(status);
    }
    let mut chosen = result;
    while !chosen.is_null() && (unsafe { (*chosen).ai_family }) != family && family != libc::AF_UNSPEC {
        chosen = unsafe { (*chosen).ai_next };
    }
    if chosen.is_null() {
        unsafe { libc::freeaddrinfo(result) };
        set_h_errno(NO_DATA);
        return Err(codes::EAI_NODATA);
    }
    let node = unsafe { &*chosen };
    let storage = unsafe { &*(node.ai_addr as *const libc::sockaddr_storage) };
    let guest = match storage.ss_family as i32 {
        libc::AF_INET => {
            let address = unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in) };
            address.sin_addr.s_addr.to_ne_bytes().to_vec()
        }
        libc::AF_INET6 => {
            let address = unsafe { &*(storage as *const libc::sockaddr_storage as *const libc::sockaddr_in6) };
            address.sin6_addr.s6_addr.to_vec()
        }
        _ => Vec::new(),
    };
    let canonical = if node.ai_canonname.is_null() { Vec::new() } else { unsafe { std::ffi::CStr::from_ptr(node.ai_canonname) }.to_bytes().to_vec() };
    let name_bytes = if canonical.is_empty() { cstr_bytes(name.as_ptr() as usize).to_vec() } else { canonical };
    unsafe { libc::freeaddrinfo(result) };
    set_h_errno(0);
    Ok(hostent_from(&name_bytes, family, &guest))
}

fn hostent_result(context: &mut CallContext, outcome: Result<usize, i32>) {
    match outcome {
        Ok(pointer) => context.ret_ptr(pointer),
        Err(code) => {
            context.set_errno(code);
            context.ret_ptr(0);
        }
    }
}

fn text_error(code: i32) -> &'static [u8] {
    // cite: bionic getaddrinfo.c#gai_strerror
    match code {
        codes::EAI_ADDRFAMILY => b"Address family for hostname not supported",
        codes::EAI_AGAIN => b"Temporary failure in name resolution",
        codes::EAI_BADFLAGS => b"Invalid value for ai_flags",
        codes::EAI_FAIL => b"Non-recoverable failure in name resolution",
        codes::EAI_FAMILY => b"ai_family not supported",
        codes::EAI_MEMORY => b"Memory allocation failure",
        codes::EAI_NODATA => b"No address associated with hostname",
        codes::EAI_NONAME => b"hostname nor servname provided, or not known",
        codes::EAI_SERVICE => b"servname not supported for ai_socktype",
        codes::EAI_SOCKTYPE => b"ai_socktype not supported",
        codes::EAI_SYSTEM => b"System error returned in errno",
        codes::EAI_BADHINTS => b"Invalid value for hints",
        codes::EAI_PROTOCOL => b"Resolved protocol is unknown",
        codes::EAI_OVERFLOW => b"Argument buffer overflow",
        _ => b"Unknown error",
    }
}

fn resolver_missing(what: &str) -> i64 {
    missing(what, "the Bionic resolver state and raw DNS transport have no host equivalent; use getaddrinfo");
    -1
}

pub fn register() {
    in_libc("getaddrinfo", |context| {
        let node = name_at(context.arg_ptr(0));
        let service = name_at(context.arg_ptr(1));
        let result = match (node, service) {
            (None, None) => Err(codes::EAI_NONAME),
            (Some(Ok(node)), Some(Ok(service))) => resolve(Some(node), Some(service), context.arg_ptr(2)),
            (Some(Ok(node)), None) => resolve(Some(node), None, context.arg_ptr(2)),
            (None, Some(Ok(service))) => resolve(None, Some(service), context.arg_ptr(2)),
            (Some(Err(code)), _) | (_, Some(Err(code))) => Err(code),
        };
        if context.arg_ptr(3) == 0 {
            finish_int(context, error(EFAULT));
            return;
        }
        match result {
            Ok(pointer) => {
                write_value::<usize>(context.arg_ptr(3), pointer);
                context.ret_i32(0);
            }
            Err(code) => {
                if code == EAI_SYSTEM_LOCAL {
                    context.set_errno(super::fs_result::host_errno());
                }
                context.ret_i32(code);
            }
        }
    });
    in_libc("freeaddrinfo", |context| free_list(context.arg_ptr(0)));
    in_libc("gai_strerror", |context| {
        let text = text_error(context.arg_i32(0));
        let pointer = super::guest_mem::intern(std::str::from_utf8(text).unwrap_or("Unknown error"));
        context.ret_ptr(pointer);
    });
    in_libc("getnameinfo", |context| {
        let code = name_info(context.arg_ptr(0), context.arg_u32(1) as usize, context.arg_ptr(2), context.arg_u64(3) as usize, context.arg_ptr(4), context.arg_u64(5) as usize, context.arg_i32(6));
        context.ret_i32(code);
    });
    in_libc("gethostbyname", |context| {
        let name = match optional_cstr(context.arg_ptr(0)) {
            Some(bytes) => CString::new(bytes).ok(),
            None => None,
        };
        hostent_result(context, lookup_host(name, libc::AF_INET));
    });
    in_libc("gethostbyname2", |context| {
        let name = match optional_cstr(context.arg_ptr(0)) {
            Some(bytes) => CString::new(bytes).ok(),
            None => None,
        };
        hostent_result(context, lookup_host(name, super::translate::family_to_host(context.arg_i32(1)).unwrap_or(libc::AF_INET)));
    });
    in_libc("gethostbyaddr", |context| {
        let (pointer, length, kind) = (context.arg_ptr(0), context.arg_u32(1) as usize, context.arg_i32(2));
        if pointer == 0 {
            set_h_errno(NO_RECOVERY);
            context.set_errno(EFAULT);
            context.ret_ptr(0);
            return;
        }
        let mut storage: libc::sockaddr_storage = unsafe { std::mem::zeroed() };
        let host_length = match kind {
            abi::AF_INET if length == 4 => {
                let address = unsafe { &mut *(&mut storage as *mut libc::sockaddr_storage as *mut libc::sockaddr_in) };
                address.sin_len = abi::SOCKADDR_SIZE as u8;
                address.sin_family = libc::AF_INET as u8;
                address.sin_addr.s_addr = read_value::<u32>(pointer);
                abi::SOCKADDR_SIZE
            }
            abi::AF_INET6 if length == 16 => {
                let address = unsafe { &mut *(&mut storage as *mut libc::sockaddr_storage as *mut libc::sockaddr_in6) };
                address.sin6_len = abi::SOCKADDR_IN6_SIZE as u8;
                address.sin6_family = libc::AF_INET6 as u8;
                unsafe { std::ptr::copy_nonoverlapping(pointer as *const u8, address.sin6_addr.s6_addr.as_mut_ptr(), 16) };
                abi::SOCKADDR_IN6_SIZE
            }
            _ => {
                set_h_errno(NO_RECOVERY);
                context.set_errno(codes::EAI_FAMILY);
                context.ret_ptr(0);
                return;
            }
        };
        let mut host = vec![0u8; codes::NI_MAXHOST];
        let status = unsafe { libc::getnameinfo(&storage as *const _ as *const libc::sockaddr, host_length as libc::socklen_t, host.as_mut_ptr() as *mut libc::c_char, host.len() as libc::socklen_t, std::ptr::null_mut(), 0, codes::NI_NAMEREQD) };
        if status != 0 {
            set_h_errno(if status == codes::EAI_AGAIN { TRY_AGAIN } else { NAME_NOT_FOUND });
            context.set_errno(status);
            context.ret_ptr(0);
            return;
        }
        let bytes = unsafe { std::ffi::CStr::from_ptr(host.as_ptr() as *const libc::c_char) }.to_bytes().to_vec();
        let address = unsafe { std::slice::from_raw_parts(pointer as *const u8, length) };
        set_h_errno(0);
        context.ret_ptr(hostent_from(&bytes, kind, address));
    });
    in_libc("__get_h_errno", |context| context.ret_ptr(h_errno_address()));
    in_libc("herror", |context| {
        let prefix = optional_cstr(context.arg_ptr(0)).map(|bytes| String::from_utf8_lossy(bytes).into_owned()).unwrap_or_default();
        let code = H_ERRNO.with(|slot| slot.get());
        super::super::host_log::line(&format!("{prefix}: resolver error {code}"));
    });
    in_libc("hstrerror", |context| {
        let message = match context.arg_i32(0) {
            1 => "Unknown host",
            2 => "Host name lookup failure",
            3 => "Unknown server error",
            4 => "No address associated with name",
            _ => "Resolver internal error",
        };
        context.ret_ptr(super::guest_mem::intern(message));
    });
    for name in ["res_init", "__res_init", "res_ninit"] {
        in_libc(name, move |context: &mut CallContext| context.ret_i32(resolver_missing(name) as i32));
    }
    for name in ["res_mkquery", "res_query", "res_search", "res_querydomain", "res_send", "res_nquery", "res_nsearch", "res_nsend"] {
        in_libc(name, move |context: &mut CallContext| context.ret_i32(resolver_missing(name) as i32));
    }
    for name in ["dn_comp", "dn_expand"] {
        in_libc(name, move |context: &mut CallContext| context.ret_i32(resolver_missing(name) as i32));
    }
    in_libc("res_close", |_| {});
}
