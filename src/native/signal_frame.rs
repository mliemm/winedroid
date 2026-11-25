use super::bionic::guest_mem::{read_value, write_value};
use super::signal_numbers::{darwin_to_linux, to_linux_mask};

pub const SIGINFO_BYTES: usize = 128;
pub const UCONTEXT_BYTES: usize = 4560;
pub const FRAME_RECORD_BYTES: usize = 16;
pub const FRAME_BYTES: usize = FRAME_RECORD_BYTES + SIGINFO_BYTES + UCONTEXT_BYTES;
pub const SIGINFO_OFFSET: usize = FRAME_RECORD_BYTES;
pub const UCONTEXT_OFFSET: usize = FRAME_RECORD_BYTES + SIGINFO_BYTES;

const UC_STACK: usize = 16;
const UC_SIGMASK: usize = 40;
const UC_FAULT_ADDRESS: usize = 176;
const UC_REGS: usize = 184;
const UC_SP: usize = 432;
const UC_PC: usize = 440;
const UC_PSTATE: usize = 448;
const UC_RESERVED: usize = 464;
const FPSIMD_MAGIC: u32 = 0x4650_8001;
const FPSIMD_SIZE: u32 = 528;
const ESR_MAGIC: u32 = 0x4553_5201;
const ESR_SIZE: u32 = 16;

const DARWIN_MCONTEXT_POINTER: usize = 48;
const DARWIN_UC_SIGMASK: usize = 4;
const DARWIN_ESR: usize = 8;
const DARWIN_FAR: usize = 0;
const DARWIN_X0: usize = 16;
const DARWIN_FP: usize = 248;
const DARWIN_LR: usize = 256;
const DARWIN_SP: usize = 264;
const DARWIN_PC: usize = 272;
const DARWIN_CPSR: usize = 280;
const DARWIN_VECTORS: usize = 288;
const DARWIN_FPSR: usize = 800;
const DARWIN_FPCR: usize = 804;
const PLATFORM_REGISTER: usize = 18;
const NZCV: u32 = 0xF000_0000;

const SI_USER: i32 = 0;
const SI_QUEUE: i32 = -1;
const SI_TIMER: i32 = -2;
const SI_MESGQ: i32 = -3;
const SI_ASYNCIO: i32 = -4;
pub const SI_TKILL: i32 = -6;

const SEGV_MAPERR: i32 = 1;
const SEGV_ACCERR: i32 = 2;
const BUS_ADRALN: i32 = 1;
const TRAP_BRKPT: i32 = 1;
const ILL_ILLOPC: i32 = 1;

extern "C" {
    fn mach_vm_region(task: u32, address: *mut u64, size: *mut u64, flavor: i32, info: *mut i32, count: *mut u32, object_name: *mut u32) -> i32;
}

#[derive(Debug, Clone, Copy)]
pub struct Origin {
    pub code: i32,
    pub pid: i32,
    pub uid: u32,
}

pub fn mcontext(context: usize) -> usize {
    read_value::<usize>(context + DARWIN_MCONTEXT_POINTER)
}

pub fn interrupted_sp(context: usize) -> u64 {
    read_value(mcontext(context) + DARWIN_SP)
}

fn address_is_mapped(address: u64) -> bool {
    let mut at = address;
    let (mut size, mut info, mut count, mut object) = (0u64, [0i32; 9], 9u32, 0u32);
    let status = unsafe { mach_vm_region(libc::mach_task_self(), &mut at, &mut size, 9, info.as_mut_ptr(), &mut count, &mut object) };
    status == 0 && at <= address
}

fn map_darwin_code(code: i32) -> i32 {
    match code {
        0x10001 => SI_USER,
        0x10002 => SI_QUEUE,
        0x10003 => SI_TIMER,
        0x10004 => SI_ASYNCIO,
        0x10005 => SI_MESGQ,
        other => other,
    }
}

pub struct Delivery {
    pub signal: i32,
    pub code: i32,
    pub address: u64,
    pub pid: i32,
    pub uid: u32,
    pub fault: bool,
}

pub fn classify(darwin_signal: i32, info: usize, context: usize) -> Option<Delivery> {
    // cite: bionic siginfo.h#SEGV_MAPERR, SEGV_ACCERR, BUS_ADRALN, TRAP_BRKPT, ILL_ILLOPC
    let signal = darwin_to_linux(darwin_signal)?;
    let code = map_darwin_code(read_value::<i32>(info + 8));
    let sender = Delivery { signal, code, address: 0, pid: read_value(info + 12), uid: read_value(info + 16), fault: false };
    let hardware = |code: i32, address: u64| Delivery { signal, code, address, pid: 0, uid: 0, fault: true };
    let context_base = mcontext(context);
    let esr: u32 = read_value(context_base + DARWIN_ESR);
    let far: u64 = read_value::<u64>(context_base + DARWIN_FAR) & 0x00FF_FFFF_FFFF_FFFF;
    let pc: u64 = read_value(context_base + DARWIN_PC);
    let memory_fault = matches!(esr >> 26, 0x20 | 0x21 | 0x24 | 0x25);
    Some(match darwin_signal {
        libc::SIGSEGV | libc::SIGBUS if code > 0 || memory_fault => {
            let address = if matches!(esr >> 26, 0x20 | 0x21) { pc } else { far };
            if esr & 0x3F == 0x21 && matches!(esr >> 26, 0x24 | 0x25) {
                Delivery { signal: 7, ..hardware(BUS_ADRALN, address) }
            } else {
                Delivery { signal: 11, ..hardware(if address_is_mapped(address) { SEGV_ACCERR } else { SEGV_MAPERR }, address) }
            }
        }
        libc::SIGTRAP if code > 0 || esr >> 26 == 0x3C => hardware(TRAP_BRKPT, pc),
        libc::SIGILL if code > 0 || esr >> 26 == 0 => hardware(ILL_ILLOPC, pc),
        libc::SIGFPE if code > 0 => hardware(code, read_value::<u64>(info + 24)),
        _ => sender,
    })
}

pub fn write_siginfo(buffer: usize, delivery: &Delivery) {
    unsafe { std::ptr::write_bytes(buffer as *mut u8, 0, SIGINFO_BYTES) };
    write_value::<i32>(buffer, delivery.signal);
    write_value::<i32>(buffer + 8, delivery.code);
    if delivery.fault {
        write_value::<u64>(buffer + 16, delivery.address);
    } else {
        write_value::<i32>(buffer + 16, delivery.pid);
        write_value::<u32>(buffer + 20, delivery.uid);
    }
}

pub struct StackRecord {
    pub base: u64,
    pub size: u64,
    pub flags: i32,
}

pub fn write_ucontext(buffer: usize, mask: u64, stack: &StackRecord, darwin_context: Option<usize>, fault_address: u64, fault_status: bool) {
    unsafe { std::ptr::write_bytes(buffer as *mut u8, 0, UCONTEXT_BYTES) };
    write_value::<u64>(buffer + UC_STACK, stack.base);
    write_value::<i32>(buffer + UC_STACK + 8, stack.flags);
    write_value::<u64>(buffer + UC_STACK + 16, stack.size);
    write_value::<u64>(buffer + UC_SIGMASK, mask);
    write_value::<u64>(buffer + UC_FAULT_ADDRESS, fault_address);
    let Some(context) = darwin_context else { return };
    let source = mcontext(context);
    for index in 0..29 {
        write_value::<u64>(buffer + UC_REGS + 8 * index, read_value(source + DARWIN_X0 + 8 * index));
    }
    write_value::<u64>(buffer + UC_REGS + 8 * 29, read_value(source + DARWIN_FP));
    write_value::<u64>(buffer + UC_REGS + 8 * 30, read_value(source + DARWIN_LR));
    write_value::<u64>(buffer + UC_SP, read_value(source + DARWIN_SP));
    write_value::<u64>(buffer + UC_PC, read_value(source + DARWIN_PC));
    write_value::<u64>(buffer + UC_PSTATE, u64::from(read_value::<u32>(source + DARWIN_CPSR)));
    let reserved = buffer + UC_RESERVED;
    write_value::<u32>(reserved, FPSIMD_MAGIC);
    write_value::<u32>(reserved + 4, FPSIMD_SIZE);
    write_value::<u32>(reserved + 8, read_value(source + DARWIN_FPSR));
    write_value::<u32>(reserved + 12, read_value(source + DARWIN_FPCR));
    unsafe { std::ptr::copy_nonoverlapping((source + DARWIN_VECTORS) as *const u8, (reserved + 16) as *mut u8, 512) };
    if fault_status {
        let esr = reserved + FPSIMD_SIZE as usize;
        write_value::<u32>(esr, ESR_MAGIC);
        write_value::<u32>(esr + 4, ESR_SIZE);
        write_value::<u64>(esr + 8, u64::from(read_value::<u32>(source + DARWIN_ESR)));
    }
}

pub fn read_back(buffer: usize, context: usize) {
    let target = mcontext(context);
    for index in (0..29).filter(|index| *index != PLATFORM_REGISTER) {
        write_value::<u64>(target + DARWIN_X0 + 8 * index, read_value(buffer + UC_REGS + 8 * index));
    }
    write_value::<u64>(target + DARWIN_FP, read_value::<u64>(buffer + UC_REGS + 8 * 29));
    write_value::<u64>(target + DARWIN_LR, read_value::<u64>(buffer + UC_REGS + 8 * 30));
    write_value::<u64>(target + DARWIN_SP, read_value::<u64>(buffer + UC_SP));
    write_value::<u64>(target + DARWIN_PC, read_value::<u64>(buffer + UC_PC));
    let cpsr: u32 = read_value(target + DARWIN_CPSR);
    let pstate = read_value::<u64>(buffer + UC_PSTATE) as u32;
    write_value::<u32>(target + DARWIN_CPSR, (cpsr & !NZCV) | (pstate & NZCV));
    let reserved = buffer + UC_RESERVED;
    if read_value::<u32>(reserved) == FPSIMD_MAGIC && read_value::<u32>(reserved + 4) == FPSIMD_SIZE {
        write_value::<u32>(target + DARWIN_FPSR, read_value::<u32>(reserved + 8));
        write_value::<u32>(target + DARWIN_FPCR, read_value::<u32>(reserved + 12));
        unsafe { std::ptr::copy_nonoverlapping((reserved + 16) as *const u8, (target + DARWIN_VECTORS) as *mut u8, 512) };
    }
}

pub fn handler_mask(buffer: usize) -> u64 {
    read_value(buffer + UC_SIGMASK)
}

pub fn restore_darwin_mask(context: usize, linux_mask: u64) {
    write_value::<u32>(context + DARWIN_UC_SIGMASK, super::signal_numbers::to_darwin_mask(linux_mask));
}

pub fn darwin_mask_of(context: usize) -> u64 {
    to_linux_mask(read_value(context + DARWIN_UC_SIGMASK))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::offset_of;

    #[repr(C)]
    struct Sigcontext {
        fault_address: u64,
        regs: [u64; 31],
        sp: u64,
        pc: u64,
        pstate: u64,
    }

    #[test]
    fn linux_aarch64_frame_offsets_match_the_bionic_headers() {
        // cite: bionic sigcontext.h#sigcontext, fpsimd_context
        assert_eq!(UC_FAULT_ADDRESS + offset_of!(Sigcontext, regs), UC_REGS);
        assert_eq!(UC_FAULT_ADDRESS + offset_of!(Sigcontext, sp), UC_SP);
        assert_eq!(UC_FAULT_ADDRESS + offset_of!(Sigcontext, pc), UC_PC);
        assert_eq!(UC_FAULT_ADDRESS + offset_of!(Sigcontext, pstate), UC_PSTATE);
        assert_eq!(UC_RESERVED, 464);
        assert_eq!((FRAME_BYTES, SIGINFO_OFFSET, UCONTEXT_OFFSET), (4704, 16, 144));
    }

    #[test]
    fn read_back_copies_every_register_the_guest_handler_may_edit_into_the_interrupted_darwin_context() {
        // cite: bionic sigcontext.h#sigcontext, fpsimd_context
        let mut linux = vec![0u64; UCONTEXT_BYTES / 8];
        let buffer = linux.as_mut_ptr() as usize;
        for index in 0..31 {
            write_value::<u64>(buffer + UC_REGS + 8 * index, 0x1000 + index as u64);
        }
        write_value::<u64>(buffer + UC_SP, 0x2000);
        write_value::<u64>(buffer + UC_PC, 0x3000);
        write_value::<u64>(buffer + UC_PSTATE, 0xA000_0000 | 0x3C5);
        let reserved = buffer + UC_RESERVED;
        write_value::<u32>(reserved, FPSIMD_MAGIC);
        write_value::<u32>(reserved + 4, FPSIMD_SIZE);
        write_value::<u32>(reserved + 8, 7);
        write_value::<u32>(reserved + 12, 9);
        for byte in 0..512 {
            write_value::<u8>(reserved + 16 + byte, (byte % 251) as u8);
        }
        let mut darwin = vec![0xEEu8; 1024];
        let target = darwin.as_mut_ptr() as usize;
        write_value::<u32>(target + DARWIN_CPSR, 0x5000_0105);
        let mut context = vec![0u64; 16];
        write_value::<usize>(context.as_mut_ptr() as usize + DARWIN_MCONTEXT_POINTER, target);

        read_back(buffer, context.as_ptr() as usize);

        for index in (0..29).filter(|index| *index != PLATFORM_REGISTER) {
            assert_eq!(read_value::<u64>(target + DARWIN_X0 + 8 * index), 0x1000 + index as u64, "x{index}");
        }
        assert_eq!(read_value::<u64>(target + DARWIN_X0 + 8 * PLATFORM_REGISTER), 0xEEEE_EEEE_EEEE_EEEE, "x18 belongs to the platform");
        assert_eq!(read_value::<u64>(target + DARWIN_FP), 0x1000 + 29);
        assert_eq!(read_value::<u64>(target + DARWIN_LR), 0x1000 + 30);
        assert_eq!(read_value::<u64>(target + DARWIN_SP), 0x2000);
        assert_eq!(read_value::<u64>(target + DARWIN_PC), 0x3000);
        assert_eq!(read_value::<u32>(target + DARWIN_CPSR), 0x0000_0105 | 0xA000_0000, "only the condition flags come from the guest");
        assert_eq!((read_value::<u32>(target + DARWIN_FPSR), read_value::<u32>(target + DARWIN_FPCR)), (7, 9));
        assert!((0..512).all(|byte| read_value::<u8>(target + DARWIN_VECTORS + byte) == (byte % 251) as u8));
    }

    #[test]
    fn read_back_leaves_the_vector_state_alone_when_the_guest_frame_has_no_fpsimd_record() {
        // cite: bionic sigcontext.h#fpsimd_context
        let mut linux = vec![0u64; UCONTEXT_BYTES / 8];
        let mut darwin = vec![0xEEu8; 1024];
        let target = darwin.as_mut_ptr() as usize;
        let mut context = vec![0u64; 16];
        write_value::<usize>(context.as_mut_ptr() as usize + DARWIN_MCONTEXT_POINTER, target);

        read_back(linux.as_mut_ptr() as usize, context.as_ptr() as usize);

        assert!((0..512).all(|byte| read_value::<u8>(target + DARWIN_VECTORS + byte) == 0xEE));
        assert_eq!(read_value::<u32>(target + DARWIN_FPSR), 0xEEEE_EEEE);
    }
}
