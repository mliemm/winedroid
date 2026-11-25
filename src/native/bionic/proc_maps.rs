use std::os::unix::fs::MetadataExt;

use super::super::guest_thread;
use super::super::runtime::NativeRuntime;
use super::super::thread_list;
use super::abi_types::PAGE_SIZE;
use super::guest_paths::to_guest;
use super::linux_abi::mman;
use super::map_registry;
use super::proc_process::primary_record;
use super::stat_convert::linux_device;
use crate::native::guest_context;
use crate::guest_app_paths;

const PT_LOAD: u32 = 1;
const PHDR_SIZE: usize = 56;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

#[derive(Clone, PartialEq)]
struct Line {
    start: usize,
    end: usize,
    prot: i32,
    shared: bool,
    offset: u64,
    device: (u32, u32),
    inode: u64,
    name: String,
}

#[repr(C)]
#[derive(Default)]
struct BasicInfo {
    protection: i32,
    max_protection: i32,
    inheritance: u32,
    shared: u32,
    reserved: u32,
    offset: u64,
    behavior: i32,
    wired: u16,
}

extern "C" {
    fn mach_vm_region(task: u32, address: *mut u64, size: *mut u64, flavor: i32, info: *mut i32, count: *mut u32, object: *mut u32) -> i32;
}

const REGION_BASIC_INFO_64: i32 = 9;

fn mach_protection(address: usize) -> Option<(usize, usize, i32)> {
    let mut start = address as u64;
    let mut size = 0u64;
    let mut info = BasicInfo::default();
    let mut count = (std::mem::size_of::<BasicInfo>() / 4) as u32;
    let mut object = 0u32;
    let status = unsafe { mach_vm_region(super::super::mach_task::current(), &mut start, &mut size, REGION_BASIC_INFO_64, &mut info as *mut _ as *mut i32, &mut count, &mut object) };
    (status == 0).then_some((start as usize, (start + size) as usize, info.protection))
}

fn linux_prot_of_mach(protection: i32) -> i32 {
    (if protection & 1 != 0 { mman::PROT_READ } else { 0 }) | (if protection & 2 != 0 { mman::PROT_WRITE } else { 0 }) | (if protection & 4 != 0 { mman::PROT_EXEC } else { 0 })
}

fn prot_of_flags(flags: u32) -> i32 {
    (if flags & PF_R != 0 { mman::PROT_READ } else { 0 }) | (if flags & PF_W != 0 { mman::PROT_WRITE } else { 0 }) | (if flags & PF_X != 0 { mman::PROT_EXEC } else { 0 })
}

fn guest_name(host: &str) -> String {
    if let Some(guest) = to_guest(host) {
        return guest;
    }
    let file = std::path::Path::new(host).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    match guest_context::current() {
        Some(context) => format!("{}/{file}", guest_app_paths::native_library_dir(&context.package)),
        None => file,
    }
}

fn image_lines() -> Vec<Line> {
    let mut lines = Vec::new();
    let Some(runtime) = NativeRuntime::current() else { return lines };
    runtime.loader().dl_iterate_phdr(&mut |info| {
        if info.phnum == 0 || info.phdr == 0 {
            return 0;
        }
        let name = guest_name(&info.name);
        let metadata = std::fs::metadata(&info.name).ok();
        let (device, inode) = metadata.map(|meta| (linux_device(meta.dev() as i32), meta.ino())).unwrap_or(((0, 0), 0));
        for index in 0..info.phnum as usize {
            let header = info.phdr + index * PHDR_SIZE;
            let kind = super::guest_mem::read_value::<u32>(header);
            if kind != PT_LOAD {
                continue;
            }
            let flags = super::guest_mem::read_value::<u32>(header + 4);
            let file_offset = super::guest_mem::read_value::<u64>(header + 8);
            let vaddr = super::guest_mem::read_value::<u64>(header + 16) as usize;
            let memsz = super::guest_mem::read_value::<u64>(header + 40) as usize;
            let begin = (info.addr + vaddr) & !(PAGE_SIZE - 1);
            let end = (info.addr + vaddr + memsz + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
            let mut cursor = begin;
            while cursor < end {
                let (region_start, region_end, protection) = mach_protection(cursor).map(|(a, b, p)| (a.max(cursor), b.min(end), linux_prot_of_mach(p))).unwrap_or((cursor, end, prot_of_flags(flags)));
                lines.push(Line { start: region_start, end: region_end, prot: protection, shared: false, offset: (file_offset & !0xfff) + (region_start - begin) as u64, device, inode, name: name.clone() });
                cursor = region_end.max(cursor + 1);
            }
        }
        0
    });
    lines
}

fn registry_lines() -> Vec<Line> {
    map_registry::snapshot()
        .into_iter()
        .map(|(start, record)| {
            let (device, inode, offset, file_name) = match &record.file {
                Some(file) => (file.device, file.inode, file.offset & !0xfff, file.guest_path.clone()),
                None => ((0, 0), 0, 0, String::new()),
            };
            let name = if !file_name.is_empty() { file_name } else { record.name.map(|name| format!("[anon:{name}]")).unwrap_or_default() };
            Line { start, end: start + record.len, prot: record.prot, shared: record.shared, offset, device, inode, name }
        })
        .collect()
}

fn stack_lines() -> Vec<Line> {
    let primary = primary_record();
    guest_thread::live_records()
        .into_iter()
        .filter_map(|(record, tid)| {
            let host = thread_list::host_of(record)?;
            let top = unsafe { libc::pthread_get_stackaddr_np(host) } as usize;
            let size = unsafe { libc::pthread_get_stacksize_np(host) };
            let name = if Some(record) == primary { "[stack]".to_string() } else { format!("[anon:stack_and_tls:{tid}]") };
            Some(Line { start: top - size, end: top, prot: mman::PROT_READ | mman::PROT_WRITE, shared: false, offset: 0, device: (0, 0), inode: 0, name })
        })
        .collect()
}

fn merge(mut lines: Vec<Line>) -> Vec<Line> {
    lines.sort_by_key(|line| line.start);
    let mut merged: Vec<Line> = Vec::new();
    for line in lines {
        if let Some(last) = merged.last_mut() {
            if line.start < last.end {
                continue;
            }
            let contiguous = last.end == line.start && last.prot == line.prot && last.shared == line.shared && last.name == line.name && last.device == line.device && last.inode == line.inode && (last.inode == 0 || last.offset + (last.end - last.start) as u64 == line.offset);
            if contiguous {
                last.end = line.end;
                continue;
            }
        }
        merged.push(line);
    }
    merged
}

fn render(line: &Line) -> String {
    let permissions: String = [
        if line.prot & mman::PROT_READ != 0 { 'r' } else { '-' },
        if line.prot & mman::PROT_WRITE != 0 { 'w' } else { '-' },
        if line.prot & mman::PROT_EXEC != 0 { 'x' } else { '-' },
        if line.shared { 's' } else { 'p' },
    ]
    .iter()
    .collect();
    let head = format!("{:08x}-{:08x} {permissions} {:08x} {:02x}:{:02x} {}", line.start, line.end, line.offset, line.device.0, line.device.1, line.inode);
    // cite: bionic stdio.cpp#__sFILE
    if line.name.is_empty() {
        format!("{head}\n")
    } else {
        format!("{head:<73} {}\n", line.name)
    }
}

pub fn maps() -> Vec<u8> {
    let mut lines = image_lines();
    lines.extend(registry_lines());
    lines.extend(stack_lines());
    merge(lines).iter().map(render).collect::<String>().into_bytes()
}
