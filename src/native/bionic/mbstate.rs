use std::sync::Mutex;

use super::abi_types::{EILSEQ, EINVAL};
use super::export::in_libc;
use super::guest_mem::{read_value, write_value};

const ILLEGAL: u64 = u64::MAX;
const INCOMPLETE: u64 = u64::MAX - 1;
const WEOF: u32 = u32::MAX;
const PRIVATE_STATES: usize = 10;
const STATE_MBRTOWC: usize = 0;
const STATE_MBRTOC32: usize = 1;
const STATE_MBRLEN: usize = 2;
const STATE_WCRTOMB: usize = 3;
const STATE_C32RTOMB: usize = 4;
const STATE_MBTOWC: usize = 5;
const STATE_WCTOMB: usize = 6;
const STATE_BTOWC: usize = 7;
const STATE_MBSNRTOWCS: usize = 8;
const STATE_WCSNRTOMBS: usize = 9;
const ASCII_LIMIT: u8 = 0x80;
const SEQUENCE_WINDOW: usize = 4;

type Sequence = [u8; 4];

static PRIVATE: Mutex<[Sequence; PRIVATE_STATES]> = Mutex::new([[0; 4]; PRIVATE_STATES]);

pub struct Outcome {
    pub value: u64,
    pub errno: Option<i32>,
}

fn illegal(errno: i32, state: &mut Sequence) -> Outcome {
    // cite: bionic bionic_mbstate.h#mbstate_reset_and_return_illegal
    *state = [0; 4];
    Outcome { value: ILLEGAL, errno: Some(errno) }
}

fn is_initial(state: &Sequence) -> bool {
    // cite: bionic bionic_mbstate.h#mbstate_is_initial
    *state == [0; 4]
}

fn bytes_so_far(state: &Sequence) -> usize {
    // cite: bionic bionic_mbstate.h#mbstate_bytes_so_far
    if state[2] != 0 {
        3
    } else if state[1] != 0 {
        2
    } else {
        usize::from(state[0] != 0)
    }
}

pub fn decode(state: &mut Sequence, input: Option<&[u8]>, output: &mut Option<u32>) -> Outcome {
    // cite: bionic mbrtoc32.cpp#mbrtoc32
    if state[3] != 0 {
        return illegal(EINVAL, state);
    }
    let (bytes, wanted_output) = match input {
        Some(bytes) => (bytes, true),
        None => (&[0u8][..], false),
    };
    if bytes.is_empty() {
        return Outcome { value: 0, errno: None };
    }
    if is_initial(state) && bytes[0] & 0x80 == 0 {
        if wanted_output {
            *output = Some(u32::from(bytes[0]));
        }
        return Outcome { value: u64::from(bytes[0] != 0), errno: None };
    }
    let so_far = bytes_so_far(state);
    let first = if so_far > 0 { state[0] } else { bytes[0] };
    let (mask, length, lower) = match first {
        byte if byte & 0xe0 == 0xc0 => (0x1f, 2usize, 0x80u32),
        byte if byte & 0xf0 == 0xe0 => (0x0f, 3, 0x800),
        byte if byte & 0xf8 == 0xf0 => (0x07, 4, 0x10000),
        _ => return illegal(EILSEQ, state),
    };
    let wanted = length - so_far;
    let mut taken = 0;
    for byte in bytes.iter().take(wanted) {
        if !is_initial(state) && byte & 0xc0 != 0x80 {
            return illegal(EILSEQ, state);
        }
        state[so_far + taken] = *byte;
        taken += 1;
    }
    if taken < wanted {
        return Outcome { value: INCOMPLETE, errno: None };
    }
    let mut code = u32::from(state[0]) & mask;
    for byte in &state[1..length] {
        code = (code << 6) | (u32::from(*byte) & 0x3f);
    }
    if code < lower || (0xd800..=0xdfff).contains(&code) || code > 0x10ffff {
        return illegal(EILSEQ, state);
    }
    if wanted_output {
        *output = Some(code);
    }
    *state = [0; 4];
    Outcome { value: if code == 0 { 0 } else { wanted as u64 }, errno: None }
}

pub fn encode(state: &mut Sequence, code: u32, output: &mut Vec<u8>) -> Outcome {
    // cite: bionic c32rtomb.cpp#c32rtomb
    if code == 0 {
        *state = [0; 4];
        output.push(0);
        return Outcome { value: 1, errno: None };
    }
    if !is_initial(state) {
        return illegal(EILSEQ, state);
    }
    let (lead, length) = match code {
        0..=0x7f => {
            output.push(code as u8);
            return Outcome { value: 1, errno: None };
        }
        0x80..=0x7ff => (0xc0u8, 2usize),
        0x800..=0xffff => (0xe0, 3),
        0x1_0000..=0x1f_ffff => (0xf0, 4),
        _ => return Outcome { value: ILLEGAL, errno: Some(EILSEQ) },
    };
    let mut bytes = vec![0u8; length];
    let mut rest = code;
    for slot in bytes.iter_mut().skip(1).rev() {
        *slot = 0x80 | (rest & 0x3f) as u8;
        rest >>= 6;
    }
    bytes[0] = lead | rest as u8;
    output.extend_from_slice(&bytes);
    *state = [0; 4];
    Outcome { value: length as u64, errno: None }
}

fn with_state<T>(address: usize, private: usize, work: impl FnOnce(&mut Sequence) -> T) -> T {
    if address == 0 {
        let mut states = PRIVATE.lock().expect("multibyte states");
        return work(&mut states[private]);
    }
    let mut state: Sequence = [read_value(address), read_value(address + 1), read_value(address + 2), read_value(address + 3)];
    let result = work(&mut state);
    for (index, byte) in state.iter().enumerate() {
        write_value::<u8>(address + index, *byte);
    }
    result
}

fn reset(private: usize) {
    PRIVATE.lock().expect("multibyte states")[private] = [0; 4];
}

fn finish(context: &mut super::super::bridge::CallContext, outcome: &Outcome) {
    if let Some(code) = outcome.errno {
        context.set_errno(code);
    }
    context.ret_u64(outcome.value);
}

fn decode_call(context: &mut super::super::bridge::CallContext, private: usize, output_address: usize, input: usize, count: usize, state_address: usize) -> Outcome {
    let mut output = None;
    let outcome = with_state(state_address, private, |state| {
        let bytes = (input != 0).then(|| unsafe { std::slice::from_raw_parts(input as *const u8, count) });
        decode(state, bytes, &mut output)
    });
    if let (Some(code), true) = (output, output_address != 0 && input != 0) {
        write_value::<u32>(output_address, code);
    }
    let _ = context;
    outcome
}

fn encode_call(destination: usize, code: u32, private: usize, state_address: usize) -> Outcome {
    let mut bytes = Vec::new();
    let outcome = if destination == 0 {
        with_state(state_address, private, |state| {
            *state = [0; 4];
        });
        Outcome { value: 1, errno: None }
    } else {
        with_state(state_address, private, |state| encode(state, code, &mut bytes))
    };
    if destination != 0 && outcome.errno.is_none() {
        super::guest_mem::write_bytes(destination, &bytes);
    }
    outcome
}

fn reset_and_return(value: usize, state: &mut Sequence) -> Outcome {
    // cite: bionic bionic_mbstate.h#mbstate_reset_and_return
    *state = [0; 4];
    Outcome { value: value as u64, errno: None }
}

fn sequence_at(address: usize, available: usize) -> &'static [u8] {
    unsafe { std::slice::from_raw_parts(address as *const u8, available.min(SEQUENCE_WINDOW)) }
}

fn mbsnrtowcs(destination: usize, source: usize, nmc: usize, len: usize, state: &mut Sequence) -> Outcome {
    // cite: bionic wchar.cpp#mbsnrtowcs
    let base = read_value::<usize>(source);
    let byte = |index: usize| read_value::<u8>(base + index);
    if nmc > 0 && bytes_so_far(state) > 0 && byte(0) < ASCII_LIMIT {
        return illegal(EILSEQ, state);
    }
    let (mut consumed, mut produced) = (0usize, 0usize);
    while consumed < nmc && (destination == 0 || produced < len) {
        let step;
        if byte(consumed) < ASCII_LIMIT {
            if destination != 0 {
                write_value::<u32>(destination + produced * 4, u32::from(byte(consumed)));
            }
            if byte(consumed) == 0 {
                if destination != 0 {
                    write_value::<usize>(source, 0);
                }
                return reset_and_return(produced, state);
            }
            step = 1;
        } else {
            let mut decoded = None;
            let outcome = decode(state, Some(sequence_at(base + consumed, nmc - consumed)), &mut decoded);
            match outcome.value {
                ILLEGAL => {
                    if destination != 0 {
                        write_value::<usize>(source, base + consumed);
                    }
                    return illegal(EILSEQ, state);
                }
                INCOMPLETE => {
                    if destination != 0 {
                        write_value::<usize>(source, base + nmc);
                    }
                    return illegal(EILSEQ, state);
                }
                0 => {
                    if destination != 0 {
                        write_value::<usize>(source, 0);
                    }
                    return reset_and_return(produced, state);
                }
                taken => {
                    if let (Some(code), true) = (decoded, destination != 0) {
                        write_value::<u32>(destination + produced * 4, code);
                    }
                    step = taken as usize;
                }
            }
        }
        consumed += step;
        produced += 1;
    }
    if destination != 0 {
        write_value::<usize>(source, base + consumed);
    }
    reset_and_return(produced, state)
}

fn wcsnrtombs(destination: usize, source: usize, nwc: usize, len: usize, state: &mut Sequence) -> Outcome {
    // cite: bionic wchar.cpp#wcsnrtombs
    if !is_initial(state) {
        return illegal(EILSEQ, state);
    }
    let base = read_value::<usize>(source);
    let wide = |index: usize| read_value::<u32>(base + index * 4);
    let (mut consumed, mut produced) = (0usize, 0usize);
    while consumed < nwc && (destination == 0 || produced < len) {
        let mut bytes = Vec::new();
        if wide(consumed) < u32::from(ASCII_LIMIT) {
            if wide(consumed) == 0 {
                if destination != 0 {
                    write_value::<u8>(destination + produced, 0);
                    write_value::<usize>(source, 0);
                }
                return Outcome { value: produced as u64, errno: None };
            }
            bytes.push(wide(consumed) as u8);
        } else {
            let outcome = encode(state, wide(consumed), &mut bytes);
            if outcome.value == ILLEGAL {
                if destination != 0 {
                    write_value::<usize>(source, base + consumed * 4);
                }
                return outcome;
            }
        }
        if destination != 0 {
            if bytes.len() > len - produced {
                break;
            }
            super::guest_mem::write_bytes(destination + produced, &bytes);
        }
        consumed += 1;
        produced += bytes.len();
    }
    if destination != 0 {
        write_value::<usize>(source, base + consumed * 4);
    }
    Outcome { value: produced as u64, errno: None }
}

fn register_strings() {
    in_libc("mbsnrtowcs", |context| {
        let (destination, source, nmc, len) = (context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize, context.arg_u64(3) as usize);
        let outcome = with_state(context.arg_ptr(4), STATE_MBSNRTOWCS, |state| mbsnrtowcs(destination, source, nmc, len, state));
        finish(context, &outcome);
    });
    in_libc("mbsrtowcs", |context| {
        let (destination, source, len) = (context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize);
        let outcome = with_state(context.arg_ptr(3), STATE_MBSNRTOWCS, |state| mbsnrtowcs(destination, source, usize::MAX, len, state));
        finish(context, &outcome);
    });
    in_libc("mbstowcs", |context| {
        // cite: bionic mbstowcs.c#mbstowcs
        let (destination, cursor) = (context.arg_ptr(0), [context.arg_ptr(1)]);
        let outcome = mbsnrtowcs(destination, cursor.as_ptr() as usize, usize::MAX, context.arg_u64(2) as usize, &mut [0; 4]);
        finish(context, &outcome);
    });
    in_libc("wcsnrtombs", |context| {
        let (destination, source, nwc, len) = (context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize, context.arg_u64(3) as usize);
        let outcome = with_state(context.arg_ptr(4), STATE_WCSNRTOMBS, |state| wcsnrtombs(destination, source, nwc, len, state));
        finish(context, &outcome);
    });
    in_libc("wcsrtombs", |context| {
        let (destination, source, len) = (context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize);
        let outcome = with_state(context.arg_ptr(3), STATE_WCSNRTOMBS, |state| wcsnrtombs(destination, source, usize::MAX, len, state));
        finish(context, &outcome);
    });
    in_libc("wcstombs", |context| {
        // cite: bionic wcstombs.c#wcstombs
        let (destination, cursor) = (context.arg_ptr(0), [context.arg_ptr(1)]);
        let outcome = wcsnrtombs(destination, cursor.as_ptr() as usize, usize::MAX, context.arg_u64(2) as usize, &mut [0; 4]);
        finish(context, &outcome);
    });
}

pub fn register() {
    register_strings();
    in_libc("mbsinit", |context| {
        let initial = context.arg_ptr(0) == 0 || with_state(context.arg_ptr(0), 0, |state| is_initial(state));
        context.ret_i32(i32::from(initial));
    });
    in_libc("mbrtowc", |context| {
        let outcome = decode_call(context, STATE_MBRTOWC, context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize, context.arg_ptr(3));
        finish(context, &outcome);
    });
    in_libc("mbrtoc32", |context| {
        let outcome = decode_call(context, STATE_MBRTOC32, context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize, context.arg_ptr(3));
        finish(context, &outcome);
    });
    in_libc("mbrlen", |context| {
        let outcome = decode_call(context, STATE_MBRLEN, 0, context.arg_ptr(0), context.arg_u64(1) as usize, context.arg_ptr(2));
        finish(context, &outcome);
    });
    in_libc("wcrtomb", |context| {
        let outcome = encode_call(context.arg_ptr(0), context.arg_u32(1), STATE_WCRTOMB, context.arg_ptr(2));
        finish(context, &outcome);
    });
    in_libc("c32rtomb", |context| {
        let outcome = encode_call(context.arg_ptr(0), context.arg_u32(1), STATE_C32RTOMB, context.arg_ptr(2));
        finish(context, &outcome);
    });
    in_libc("mbtowc", |context| {
        // cite: bionic mbtowc.c#mbtowc
        if context.arg_ptr(1) == 0 {
            reset(STATE_MBTOWC);
            context.ret_i32(0);
            return;
        }
        let outcome = decode_call(context, STATE_MBTOWC, context.arg_ptr(0), context.arg_ptr(1), context.arg_u64(2) as usize, 0);
        match outcome.value {
            INCOMPLETE => {
                context.set_errno(EILSEQ);
                context.ret_i32(-1);
            }
            ILLEGAL => {
                finish(context, &outcome);
                context.ret_i32(-1);
            }
            value => context.ret_i32(value as i32),
        }
    });
    in_libc("mblen", |context| {
        let outcome = decode_call(context, STATE_MBRLEN, 0, context.arg_ptr(0), context.arg_u64(1) as usize, 0);
        reset(STATE_MBRLEN);
        if context.arg_ptr(0) == 0 {
            context.ret_i32(0);
        } else {
            context.ret_i32(outcome.value as i32);
        }
    });
    in_libc("wctomb", |context| {
        // cite: bionic wctomb.c#wctomb
        if context.arg_ptr(0) == 0 {
            reset(STATE_WCTOMB);
            context.ret_i32(0);
            return;
        }
        let outcome = encode_call(context.arg_ptr(0), context.arg_u32(1), STATE_WCTOMB, 0);
        if outcome.value == ILLEGAL {
            context.set_errno(EILSEQ);
        }
        context.ret_i32(outcome.value as i32);
    });
    in_libc("btowc", |context| {
        let character = context.arg_i32(0);
        if character == -1 {
            context.ret_u32(WEOF);
            return;
        }
        let byte = [character as u8];
        let mut output = None;
        let outcome = with_state(0, STATE_BTOWC, |state| {
            let result = decode(state, Some(&byte), &mut output);
            *state = [0; 4];
            result
        });
        context.ret_u32(if outcome.value > 1 { WEOF } else { output.unwrap_or(WEOF) });
    });
    in_libc("wctob", |context| {
        let code = context.arg_u32(0);
        if code == WEOF {
            context.ret_i32(-1);
            return;
        }
        let mut bytes = Vec::new();
        let mut state: Sequence = [0; 4];
        let outcome = encode(&mut state, code, &mut bytes);
        context.ret_i32(if outcome.value == 1 { i32::from(bytes[0]) } else { -1 });
    });
}
