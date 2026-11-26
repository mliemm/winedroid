use super::bridge_test_support::{register_all, take_probe, Bridged, Seen};
use super::guest_regs::Regs;
use super::quad::f64_to_quad;
use super::va_list::GuestVaList;

#[test]
fn aapcs64_variadic_call_reaches_the_handler_through_registers_then_the_stack() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    assert_eq!(
        bridged.call("fx_call_probe_mixed").invoke().unwrap().i32(),
        1
    );

    let mut expected = vec![
        Seen::Int(2),
        Seen::Real(3.0),
        Seen::Int(4),
        Seen::Real(5.0),
        Seen::Quad(f64_to_quad(6.0)),
    ];
    expected.extend((101..=113).map(Seen::Int));
    expected.extend((201..=210).map(|value| Seen::Real(f64::from(value))));
    expected.push(Seen::Quad(f64_to_quad(7.0)));
    assert_eq!(take_probe(), expected);
}

#[test]
fn guest_va_list_passed_by_reference_twice_is_read_twice_without_write_back() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    let result = bridged
        .call("fx_call_vprobe")
        .i32(3)
        .i32(10)
        .i32(20)
        .i32(30)
        .invoke()
        .unwrap()
        .u64();

    assert_eq!(result, 60 * 100000 + 60);
}

#[test]
fn guest_va_list_arguments_beyond_the_registers_come_from_the_stack() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let mut call = bridged.call("fx_call_vprobe").i32(12);
    for value in 1..=12 {
        call = call.i32(value);
    }

    assert_eq!(call.invoke().unwrap().u64(), 78 * 100000 + 78);
}

#[test]
fn builder_assigns_integer_arguments_to_registers_then_stack_for_a_guest_variadic_callee() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    for count in [0, 3, 7, 8, 9, 12] {
        let mut call = bridged.call("fx_va_sum_int").i32(count);
        for value in 1..=count {
            call = call.i32(value * 3);
        }
        let expected: i32 = (1..=count).map(|value| value * 3).sum();
        assert_eq!(
            call.invoke().unwrap().u64() as i32,
            expected,
            "{count} ints"
        );
    }
    let mut longs = bridged.call("fx_va_sum_long").i32(9);
    for value in 0..9u64 {
        longs = longs.int(value << 33);
    }
    assert_eq!(longs.invoke().unwrap().u64(), 36 << 33);
}

#[test]
fn builder_assigns_floating_arguments_to_vector_registers_then_stack() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    for count in [7, 8, 9, 10] {
        let mut call = bridged.call("fx_va_sum_dbl").i32(count);
        for value in 1..=count {
            call = call.f64(f64::from(value) * 0.5);
        }
        let expected: f64 = (1..=count).map(|value| f64::from(value) * 0.5).sum();
        assert_eq!(call.invoke().unwrap().f64(), expected, "{count} doubles");
    }
    let promoted = bridged
        .call("fx_va_sum_dbl")
        .i32(2)
        .variadic_f32(1.5)
        .variadic_f32(2.25)
        .invoke()
        .unwrap()
        .f64();
    assert_eq!(promoted, 3.75);
}

#[test]
fn builder_places_long_double_in_a_vector_register_or_a_sixteen_byte_aligned_stack_slot() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");

    let in_register = bridged
        .call("fx_va_ld_last")
        .i32(2)
        .quad(f64_to_quad(1.5))
        .quad(f64_to_quad(9.0))
        .invoke()
        .unwrap()
        .quad();
    assert_eq!(in_register, f64_to_quad(9.0));

    let mut spilled = bridged.call("fx_va_ld_last").i32(10);
    for value in 1..=10 {
        spilled = spilled.quad(f64_to_quad(f64::from(value)));
    }
    assert_eq!(
        spilled.invoke().unwrap().quad(),
        f64_to_quad(10.0),
        "q1 to q7 then the aligned stack"
    );

    for integers in [7u64, 8] {
        let mut hole = bridged.call("fx_va_mixed_ld").i32(integers as i32).i32(9);
        for value in 1..=integers {
            hole = hole.int(value);
        }
        for value in 1..=9 {
            hole = hole.quad(f64_to_quad(f64::from(value) * 11.0));
        }
        assert_eq!(
            hole.invoke().unwrap().quad(),
            f64_to_quad(99.0),
            "{integers} ints put an {} stack word count before the long double",
            if integers % 2 == 1 { "odd" } else { "even" }
        );
    }
}

#[test]
fn builder_passes_small_homogeneous_and_large_composites_as_aapcs64_defines() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let s16: Vec<u8> = [4i64, 5]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let s24: Vec<u8> = [1i64, 2, 3]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let pair: Vec<u8> = [1.5f64, 2.0]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();

    assert_eq!(
        bridged
            .call("fx_va_s16")
            .i32(2)
            .composite(&s16, 8, None)
            .composite(&s16, 8, None)
            .invoke()
            .unwrap()
            .u64(),
        90
    );
    assert_eq!(
        bridged
            .call("fx_va_s24")
            .i32(1)
            .composite(&s24, 8, None)
            .invoke()
            .unwrap()
            .u64(),
        123
    );
    assert_eq!(
        bridged
            .call("fx_va_hfa")
            .i32(2)
            .composite(&pair, 8, Some((8, 2)))
            .composite(&pair, 8, Some((8, 2)))
            .invoke()
            .unwrap()
            .f64(),
        34.0
    );
    let mut spilled = bridged.call("fx_va_s16").i32(6);
    for _ in 0..6 {
        spilled = spilled.composite(&s16, 8, None);
    }
    assert_eq!(
        spilled.invoke().unwrap().u64(),
        6 * 45,
        "composites that no longer fit the registers go wholly to the stack"
    );
}

#[test]
fn builder_aligns_int128_to_an_even_register_pair() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let value = (3u128 << 64) | 5;

    let result = bridged
        .call("fx_va_i128")
        .i32(1)
        .int128(value)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(result, 5 * 10 + 3);
    let after_odd = bridged
        .call("fx_va_i128")
        .i32(2)
        .int128(value)
        .int128(value)
        .invoke()
        .unwrap()
        .u64();
    assert_eq!(after_odd, 2 * 53);
}

fn regs_with_stack(stack: &[u64]) -> Regs {
    let mut regs = Regs::zeroed();
    regs.sp = stack.as_ptr() as u64;
    regs
}

#[test]
fn va_list_exhausting_the_general_registers_switches_to_the_stack_and_stays_there() {
    let stack = [100u64, 101, 102, 103];
    let mut regs = regs_with_stack(&stack);
    for (index, slot) in regs.x.iter_mut().enumerate() {
        *slot = index as u64 + 10;
    }
    let mut va = GuestVaList::from_regs(&regs, 1, 0);

    let taken: Vec<u64> = (0..10).map(|_| va.next_gp()).collect();

    assert_eq!(taken, vec![11, 12, 13, 14, 15, 16, 17, 100, 101, 102]);
    assert_eq!(
        va.raw().gr_offs,
        0,
        "the offset stays at zero once the registers are used up"
    );
}

#[test]
fn va_list_long_double_after_a_general_overflow_still_uses_a_free_vector_slot() {
    let stack = [1u64, 2, 3, 4];
    let mut regs = regs_with_stack(&stack);
    regs.q[2] = [0x1111, 0x2222];
    let mut va = GuestVaList::from_regs(&regs, 8, 2);

    assert_eq!(va.next_gp(), 1);
    assert_eq!(va.next_quad(), (0x2222u128 << 64) | 0x1111);
}

#[test]
fn va_list_long_double_on_an_eight_byte_aligned_stack_skips_to_sixteen() {
    let stack = Box::new([[0u64; 2]; 3]);
    let base = stack.as_ptr() as u64;
    let mut regs = Regs::zeroed();
    regs.sp = base + 8;
    unsafe { std::ptr::write((base + 16) as *mut u128, 0xABCDu128 << 64 | 0x1234) };
    let mut va = GuestVaList::from_regs(&regs, 8, 8);

    assert_eq!(va.next_quad(), 0xABCDu128 << 64 | 0x1234);
    assert_eq!(va.raw().stack, base + 32);
}

#[test]
fn va_list_hfa_that_does_not_fit_the_vector_registers_moves_to_the_stack_and_exhausts_them() {
    let stack = [
        1.5f64.to_bits(),
        2.5f64.to_bits(),
        3.5f64.to_bits(),
        4.5f64.to_bits(),
    ];
    let regs = regs_with_stack(&stack);
    let mut va = GuestVaList::from_regs(&regs, 0, 6);

    let bytes = va.next_composite(32, 8, Some((8, 4)));

    assert_eq!(
        bytes,
        stack
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect::<Vec<u8>>()
    );
    assert!(
        va.raw().vr_offs >= 0,
        "the failed vector reservation still advances the offset past zero"
    );
}

#[test]
fn va_list_from_guest_refuses_malformed_structs() {
    register_all();
    let mut raw = [0u64; 4];
    let base = raw.as_mut_ptr() as u64;
    raw[1] = base;
    raw[2] = base;
    let write = |raw: &mut [u64; 4], gr: i32, vr: i32| {
        raw[3] = u64::from(gr as u32) | (u64::from(vr as u32) << 32)
    };

    write(&mut raw, -56, -128);
    assert!(GuestVaList::from_guest(base).is_ok());
    write(&mut raw, -3, -128);
    assert!(
        GuestVaList::from_guest(base).is_err(),
        "gr_offs not a multiple of 8"
    );
    write(&mut raw, -72, -128);
    assert!(
        GuestVaList::from_guest(base).is_err(),
        "gr_offs below the save area"
    );
    write(&mut raw, -56, -8);
    assert!(
        GuestVaList::from_guest(base).is_err(),
        "vr_offs not a multiple of 16"
    );
    write(&mut raw, -56, -144);
    assert!(
        GuestVaList::from_guest(base).is_err(),
        "vr_offs below the save area"
    );
    assert!(GuestVaList::from_guest(0).is_err(), "null list");
    raw[1] = 0;
    write(&mut raw, -56, -128);
    assert!(
        GuestVaList::from_guest(base).is_err(),
        "register area missing"
    );
}

fn numbered_general_registers() -> Regs {
    let mut regs = Regs::zeroed();
    for (index, slot) in regs.x.iter_mut().enumerate() {
        *slot = index as u64 + 10;
    }
    regs
}

fn words(values: &[u64]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[test]
fn va_list_hfa_in_vector_registers_takes_one_sixteen_byte_slot_per_member() {
    let mut regs = Regs::zeroed();
    for (index, slot) in regs.q.iter_mut().enumerate() {
        *slot = [0x1000 + index as u64, 0xFFFF_FFFF_FFFF_FFFF];
    }
    let mut va = GuestVaList::from_regs(&regs, 0, 2);

    let doubles = va.next_composite(16, 8, Some((8, 2)));

    assert_eq!(doubles, words(&[0x1002, 0x1003]));
    assert_eq!(va.raw().vr_offs, -64);
    assert_eq!(
        va.next_f64().to_bits(),
        0x1004,
        "the next vector argument is the register after the HFA"
    );

    let mut va = GuestVaList::from_regs(&regs, 1, 1);
    let floats = va.next_composite(12, 4, Some((4, 3)));
    let expected: Vec<u8> = [0x1001u32, 0x1002, 0x1003]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    assert_eq!(floats, expected);
    assert_eq!(va.raw().vr_offs, -64);
}

#[test]
fn va_list_composite_in_general_registers_takes_one_slot_per_eight_bytes() {
    let regs = numbered_general_registers();
    let mut va = GuestVaList::from_regs(&regs, 2, 0);

    let bytes = va.next_composite(12, 8, None);

    assert_eq!(bytes, words(&[12, 13])[..12]);
    assert_eq!(va.raw().gr_offs, -32, "two slots consumed for twelve bytes");
    assert_eq!(va.next_gp(), 14);
}

#[test]
fn va_list_sixteen_byte_aligned_composite_starts_on_an_even_general_register() {
    let regs = numbered_general_registers();

    let mut aligned = GuestVaList::from_regs(&regs, 1, 0);
    assert_eq!(aligned.next_composite(16, 16, None), words(&[12, 13]));
    assert_eq!(aligned.raw().gr_offs, -32);

    let mut natural = GuestVaList::from_regs(&regs, 1, 0);
    assert_eq!(natural.next_composite(16, 8, None), words(&[11, 12]));
    assert_eq!(natural.raw().gr_offs, -40);
}

#[test]
fn va_list_composite_that_overruns_the_general_registers_goes_wholly_to_the_stack() {
    let stack = [900u64, 901, 902];
    let mut regs = regs_with_stack(&stack);
    regs.x = numbered_general_registers().x;
    let mut va = GuestVaList::from_regs(&regs, 7, 0);

    let bytes = va.next_composite(16, 8, None);

    assert_eq!(bytes, words(&[900, 901]));
    assert!(
        va.raw().gr_offs >= 0,
        "later general arguments no longer use the registers"
    );
    assert_eq!(va.next_gp(), 902);
}

#[test]
fn va_list_composite_larger_than_sixteen_bytes_is_passed_by_pointer() {
    let storage = [1u64, 2, 3];
    let mut regs = Regs::zeroed();
    regs.x[0] = storage.as_ptr() as u64;
    let mut va = GuestVaList::from_regs(&regs, 0, 0);

    assert_eq!(va.next_composite(24, 8, None), words(&storage));
    assert_eq!(va.raw().gr_offs, -56, "one pointer slot");
}
