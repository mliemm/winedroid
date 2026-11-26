use std::ffi::CString;

use super::bridge_test_support::{take_shapes, Bridged};

fn pattern(index: usize, size: usize) -> Vec<u8> {
    (0..size)
        .map(|byte| ((index + 1) * 16 + byte) as u8)
        .collect()
}

fn run(shapes: &str) -> (Vec<Vec<u8>>, Vec<(i32, i32)>) {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let text = CString::new(shapes).unwrap();
    let mut call = bridged.call("fx_call_vshapes").ptr(text.as_ptr() as usize);
    let mut passed = Vec::new();
    for (index, shape) in shapes.chars().enumerate() {
        let (size, align, hfa) = match shape {
            'i' => (8, 8, None),
            'A' => (16, 16, None),
            'S' => (16, 8, None),
            'H' => (16, 8, Some((8, 2))),
            'F' => (12, 4, Some((4, 3))),
            'B' => (32, 8, None),
            other => panic!("unknown shape {other}"),
        };
        let bytes = pattern(index, size);
        call = if shape == 'i' {
            call.int(u64::from_le_bytes(bytes[..8].try_into().unwrap()))
        } else {
            call.composite(&bytes, align, hfa)
        };
        passed.push(bytes);
    }
    assert_eq!(call.invoke().unwrap().u64(), shapes.len() as u64);
    let (seen, offsets) = take_shapes();
    assert_eq!(
        seen, passed,
        "{shapes}: every item reads back the bytes the caller passed"
    );
    (seen, offsets)
}

const START: (i32, i32) = (-56, -128);

#[test]
fn aapcs64_va_arg_rounds_gr_offs_up_to_sixteen_for_a_sixteen_byte_aligned_composite() {
    assert_eq!(
        run("A").1,
        [(-32, START.1)],
        "gr_offs -56 rounds to -48 then takes two slots"
    );
    assert_eq!(
        run("iA").1,
        [(-48, START.1), (-32, START.1)],
        "-48 is already a multiple of 16"
    );
    assert_eq!(
        run("iiA").1,
        [(-48, START.1), (-40, START.1), (-16, START.1)],
        "-40 rounds to -32 then takes two slots"
    );
}

#[test]
fn aapcs64_va_arg_leaves_gr_offs_alone_for_an_eight_byte_aligned_sixteen_byte_composite() {
    assert_eq!(
        run("S").1,
        [(-40, START.1)],
        "no rounding: the composite takes x1 and x2"
    );
    assert_eq!(run("iS").1, [(-48, START.1), (-32, START.1)]);
}

#[test]
fn aapcs64_va_arg_reads_a_homogeneous_float_aggregate_from_consecutive_sixteen_byte_vector_slots() {
    assert_eq!(
        run("HH").1,
        [(START.0, -96), (START.0, -64)],
        "two double members take two q slots each time"
    );
    assert_eq!(
        run("F").1,
        [(START.0, -80)],
        "three float members take three q slots"
    );
    assert_eq!(run("FF").1, [(START.0, -80), (START.0, -32)]);
}

#[test]
fn aapcs64_va_arg_takes_an_hfa_from_the_stack_and_keeps_the_overflowed_offset_when_the_vector_slots_run_out(
) {
    assert_eq!(
        run("FFF").1,
        [(START.0, -80), (START.0, -32), (START.0, 16)],
        "-32 plus three slots is above zero"
    );
    assert_eq!(
        run("HHHHH").1.last(),
        Some(&(START.0, 0)),
        "the fifth pair finds vr_offs at zero and reads the stack"
    );
}

#[test]
fn aapcs64_va_arg_mixes_general_vector_and_indirect_items_without_crossing_the_two_counters() {
    assert_eq!(
        run("HiAH").1,
        [(START.0, -96), (-48, -96), (-32, -96), (-32, -64)]
    );
    assert_eq!(
        run("BiB").1,
        [(-48, START.1), (-40, START.1), (-32, START.1)],
        "a 32 byte struct is one pointer slot"
    );
}

#[test]
fn aapcs64_va_arg_aligns_a_sixteen_byte_composite_on_the_stack_after_an_odd_number_of_stack_words()
{
    let offsets = run("iiiiiiiiA").1;

    assert_eq!(offsets[6], (0, START.1), "seven integers use x1 to x7");
    assert_eq!(
        offsets[7],
        (0, START.1),
        "the eighth integer comes from the stack"
    );
    assert_eq!(
        offsets[8],
        (0, START.1),
        "the stack composite does not move the register offsets"
    );
}
