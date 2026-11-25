use std::arch::asm;
use std::sync::Once;

use super::bridge::{handler_address, register_handler};
use super::call_assign::CallBuilder;

extern "C" {
    fn sys_icache_invalidate(start: *mut libc::c_void, length: usize);
}

const RETURN_X8: [u32; 2] = [0xaa0803e0, 0xd65f03c0];
const CLOBBER_CALLEE_SAVED_THEN_CALL_X0: [u32; 20] = [
    0xaa0003e9, 0xd2800233, 0xd2800254, 0xd2800275, 0xd2800296, 0xd28002b7, 0xd28002d8, 0xd28002f9, 0xd280031a, 0xd280033b, 0xd280035c, 0x1e6e1008,
    0x1e6e1009, 0x1e6e100a, 0x1e6e100b, 0x1e6e100c, 0x1e6e100d, 0x1e6e100e, 0x1e6e100f, 0xd63f0120,
];

fn executable(words: &[u32]) -> usize {
    let length = 16384;
    let region = unsafe { libc::mmap(std::ptr::null_mut(), length, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_PRIVATE | libc::MAP_ANON, -1, 0) };
    assert_ne!(region, libc::MAP_FAILED);
    unsafe {
        std::ptr::copy_nonoverlapping(words.as_ptr(), region as *mut u32, words.len());
        assert_eq!(libc::mprotect(region, length, libc::PROT_READ | libc::PROT_EXEC), 0);
        sys_icache_invalidate(region, length);
    }
    region as usize
}

fn register_unwind_probe() -> usize {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register_handler("fx_unwind_probe", |context| context.fail("unwind probe"));
    });
    handler_address("fx_unwind_probe").unwrap()
}

extern "C" fn unwinding_call(stub: usize, code: usize) {
    assert!(CallBuilder::new(code).ptr(stub).invoke().is_err());
}

#[test]
fn a_host_to_guest_call_passes_the_sret_destination_in_x8() {
    let code = executable(&RETURN_X8);
    let mut destination = [0u64; 3];

    let returned = CallBuilder::new(code).sret(destination.as_mut_ptr() as *mut u8).invoke().unwrap().u64();

    assert_eq!(returned, destination.as_mut_ptr() as u64);
}

#[test]
fn host_callee_saved_registers_survive_a_guest_call_that_unwinds_after_clobbering_them() {
    let stub = register_unwind_probe();
    let code = executable(&CLOBBER_CALLEE_SAVED_THEN_CALL_X0);
    let float_seed: [u64; 8] = std::array::from_fn(|index| 0x4000_0000_0000_0000 + index as u64);
    let int_seed: [u64; 7] = std::array::from_fn(|index| 0x7700 + index as u64);
    let mut floats = [0u64; 8];
    let mut ints = [0u64; 7];

    unsafe {
        asm!(
            "ldp d8, d9, [x9]",
            "ldp d10, d11, [x9, #16]",
            "ldp d12, d13, [x9, #32]",
            "ldp d14, d15, [x9, #48]",
            "ldp x20, x21, [x10]",
            "ldp x22, x23, [x10, #16]",
            "ldp x24, x25, [x10, #32]",
            "ldr x28, [x10, #48]",
            "mov x0, x12",
            "mov x1, x13",
            "blr x11",
            "stp d8, d9, [x26]",
            "stp d10, d11, [x26, #16]",
            "stp d12, d13, [x26, #32]",
            "stp d14, d15, [x26, #48]",
            "stp x20, x21, [x27]",
            "stp x22, x23, [x27, #16]",
            "stp x24, x25, [x27, #32]",
            "str x28, [x27, #48]",
            in("x9") float_seed.as_ptr(),
            in("x10") int_seed.as_ptr(),
            in("x11") unwinding_call as extern "C" fn(usize, usize),
            in("x12") stub,
            in("x13") code,
            inout("x26") floats.as_mut_ptr() => _,
            inout("x27") ints.as_mut_ptr() => _,
            out("x20") _, out("x21") _, out("x22") _, out("x23") _, out("x24") _, out("x25") _, out("x28") _,
            out("v8") _, out("v9") _, out("v10") _, out("v11") _, out("v12") _, out("v13") _, out("v14") _, out("v15") _,
            clobber_abi("C"),
        );
    }

    assert_eq!(floats, float_seed);
    assert_eq!(ints, int_seed);
}
