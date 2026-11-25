use std::cmp::Ordering;

use super::abi_types::ERANGE;
use super::export::in_libc;
use super::guest_mem::{cstr_bytes, read_value, write_value};
use super::super::bridge::CallContext;

const MANTISSA_BITS: u32 = 112;
const EXPONENT_BIAS: i32 = 16383;
const MIN_EXPONENT: i32 = -16382;
const MAX_EXPONENT: i32 = 16383;
const SIGNIFICAND_BITS: u32 = 113;
const DIGIT_LIMIT: usize = 768;
const EXPONENT_LIMIT: i32 = 20000;
const DECIMAL_RANGE: i64 = 5100;
const BINARY_RANGE: i64 = 70000;
const CHUNK_DIGITS: usize = 9;
const CHUNK_SCALE: u32 = 1_000_000_000;
const HEX_CHUNK_DIGITS: usize = 7;
const QUIET_NAN: u128 = 0x7fff_8000_0000_0000_0000_0000_0000_0000;
const INFINITY_BITS: u128 = 0x7fff_0000_0000_0000_0000_0000_0000_0000;
const WIDE_NUMERIC: &[u8] = b"-+0123456789.xXeEpP()nNaAiIfFtTyY";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Big(Vec<u32>);

impl Big {
    fn zero() -> Self {
        Self(Vec::new())
    }

    fn one() -> Self {
        Self(vec![1])
    }

    fn from_digits(digits: &[u8], radix: u32) -> Self {
        let chunk_digits = if radix == 10 { CHUNK_DIGITS } else { HEX_CHUNK_DIGITS };
        let mut value = Self::zero();
        let mut index = 0;
        while index < digits.len() {
            let take = (digits.len() - index).min(chunk_digits);
            let mut chunk = 0u32;
            for digit in &digits[index..index + take] {
                chunk = chunk * radix + u32::from(*digit);
            }
            let mut scale = 1u32;
            for _ in 0..take {
                scale *= radix;
            }
            value.mul_small(scale);
            value.add_small(chunk);
            index += take;
        }
        value
    }

    fn power_of_two(bits: usize) -> Self {
        let mut value = Self::zero();
        value.0.resize(bits / 32 + 1, 0);
        value.0[bits / 32] = 1u32 << (bits % 32);
        value
    }

    fn mul_small(&mut self, factor: u32) {
        let mut carry = 0u64;
        for limb in self.0.iter_mut() {
            let product = u64::from(*limb) * u64::from(factor) + carry;
            *limb = product as u32;
            carry = product >> 32;
        }
        while carry != 0 {
            self.0.push(carry as u32);
            carry >>= 32;
        }
        self.normalize();
    }

    fn add_small(&mut self, addend: u32) {
        let mut carry = u64::from(addend);
        let mut index = 0;
        while carry != 0 {
            if index == self.0.len() {
                self.0.push(0);
            }
            let sum = u64::from(self.0[index]) + carry;
            self.0[index] = sum as u32;
            carry = sum >> 32;
            index += 1;
        }
        self.normalize();
    }

    fn normalize(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn bit_length(&self) -> usize {
        match self.0.last() {
            Some(top) => (self.0.len() - 1) * 32 + (32 - top.leading_zeros() as usize),
            None => 0,
        }
    }

    fn shifted(&self, bits: usize) -> Self {
        if self.is_zero() {
            return Self::zero();
        }
        let words = bits / 32;
        let offset = bits % 32;
        let mut value = Self::zero();
        value.0.resize(words, 0);
        let mut carry = 0u32;
        for limb in &self.0 {
            value.0.push((limb << offset) | carry);
            carry = if offset == 0 { 0 } else { limb >> (32 - offset) };
        }
        if carry != 0 {
            value.0.push(carry);
        }
        value.normalize();
        value
    }

    fn shr(&mut self, bits: usize) {
        let words = bits / 32;
        let offset = bits % 32;
        if words >= self.0.len() {
            self.0.clear();
            return;
        }
        self.0.drain(..words);
        if offset != 0 {
            let mut carry = 0u32;
            for limb in self.0.iter_mut().rev() {
                let next = *limb << (32 - offset);
                *limb = (*limb >> offset) | carry;
                carry = next;
            }
        }
        self.normalize();
    }

    fn compare(&self, other: &Self) -> Ordering {
        if self.0.len() != other.0.len() {
            return self.0.len().cmp(&other.0.len());
        }
        for index in (0..self.0.len()).rev() {
            if self.0[index] != other.0[index] {
                return self.0[index].cmp(&other.0[index]);
            }
        }
        Ordering::Equal
    }

    fn subtract(&mut self, other: &Self) {
        let mut borrow = 0i64;
        for index in 0..self.0.len() {
            let minuend = i64::from(self.0[index]);
            let subtrahend = i64::from(*other.0.get(index).unwrap_or(&0));
            let difference = minuend - subtrahend - borrow;
            if difference < 0 {
                self.0[index] = (difference + (1i64 << 32)) as u32;
                borrow = 1;
            } else {
                self.0[index] = difference as u32;
                borrow = 0;
            }
        }
        self.normalize();
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Magnitude {
    Zero,
    Finite { numerator: Big, denominator: Big, sticky: bool },
    Infinite,
    Overflow,
    Underflow,
    Nan,
}

#[derive(Debug, PartialEq, Eq)]
struct Scan {
    negative: bool,
    magnitude: Magnitude,
    end: usize,
}

fn matches(input: &[u8], index: usize, word: &[u8]) -> bool {
    input.len() >= index + word.len() && input[index..index + word.len()].iter().map(|byte| byte.to_ascii_lowercase()).eq(word.iter().copied())
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn digit_value(byte: u8, radix: u32) -> Option<u32> {
    let value = match byte {
        b'0'..=b'9' => u32::from(byte - b'0'),
        b'a'..=b'f' => u32::from(byte - b'a') + 10,
        b'A'..=b'F' => u32::from(byte - b'A') + 10,
        _ => return None,
    };
    (value < radix).then_some(value)
}

fn scan_decimal_exponent(input: &[u8], cursor: &mut usize) -> Option<i32> {
    if !matches!(input.get(*cursor).copied(), Some(b'e' | b'E')) {
        return Some(0);
    }
    let mut at = *cursor + 1;
    let mut negative = false;
    match input.get(at).copied() {
        Some(b'+') => at += 1,
        Some(b'-') => {
            negative = true;
            at += 1;
        }
        _ => {}
    }
    let digits = at;
    let mut value: i32 = 0;
    while let Some(byte) = input.get(at).filter(|byte| byte.is_ascii_digit()) {
        value = value.saturating_mul(10).saturating_add(i32::from(*byte - b'0')).min(EXPONENT_LIMIT);
        at += 1;
    }
    if at == digits {
        return Some(0);
    }
    *cursor = at;
    Some(if negative { -value } else { value })
}

fn significant_digits(digits: &[u8]) -> (usize, bool) {
    let leading = digits.iter().position(|digit| *digit != 0).unwrap_or(digits.len());
    let significant = &digits[leading..];
    let kept = significant.len().min(DIGIT_LIMIT);
    (kept, significant[kept..].iter().any(|digit| *digit != 0))
}

fn scan_decimal(input: &[u8], start: usize) -> Option<(Magnitude, usize)> {
    let mut cursor = start;
    let mut digits: Vec<u8> = Vec::new();
    let mut fraction_digits = 0i32;
    while let Some(byte) = input.get(cursor).copied().filter(|byte| byte.is_ascii_digit()) {
        digits.push(byte - b'0');
        cursor += 1;
    }
    let mut saw_digit = !digits.is_empty();
    if input.get(cursor) == Some(&b'.') {
        let before = digits.len();
        let dot = cursor;
        cursor += 1;
        while let Some(byte) = input.get(cursor).copied().filter(|byte| byte.is_ascii_digit()) {
            digits.push(byte - b'0');
            saw_digit = true;
            cursor += 1;
        }
        if !saw_digit {
            cursor = dot;
        } else {
            fraction_digits = (digits.len() - before) as i32;
        }
    }
    if !saw_digit {
        return None;
    }
    let mut exponent = scan_decimal_exponent(input, &mut cursor)? - fraction_digits;
    let (kept, sticky) = significant_digits(&digits);
    if kept == 0 {
        return Some((Magnitude::Zero, cursor));
    }
    let leading = digits.iter().position(|digit| *digit != 0).unwrap_or(digits.len());
    let magnitude_exponent = i64::from(exponent) + kept as i64;
    if magnitude_exponent > DECIMAL_RANGE {
        return Some((Magnitude::Overflow, cursor));
    }
    if magnitude_exponent < -DECIMAL_RANGE {
        return Some((Magnitude::Underflow, cursor));
    }
    let significant = &digits[leading..leading + kept];
    let mut numerator = Big::from_digits(significant, 10);
    let mut denominator = Big::one();
    if exponent >= 0 {
        let mut remaining = exponent as usize;
        while remaining >= CHUNK_DIGITS {
            numerator.mul_small(CHUNK_SCALE);
            remaining -= CHUNK_DIGITS;
        }
        if remaining != 0 {
            numerator.mul_small(10u32.pow(remaining as u32));
        }
    } else {
        while exponent <= -(CHUNK_DIGITS as i32) {
            denominator.mul_small(CHUNK_SCALE);
            exponent += CHUNK_DIGITS as i32;
        }
        if exponent != 0 {
            denominator.mul_small(10u32.pow((-exponent) as u32));
        }
    }
    Some((Magnitude::Finite { numerator, denominator, sticky }, cursor))
}

fn scan_hex(input: &[u8], start: usize) -> Option<(Magnitude, usize)> {
    if input.get(start) != Some(&b'0') || !matches!(input.get(start + 1).copied(), Some(b'x' | b'X')) {
        return None;
    }
    let mut cursor = start + 2;
    let mut digits: Vec<u8> = Vec::new();
    let mut fraction_digits = 0i64;
    while let Some(value) = input.get(cursor).copied().and_then(|byte| digit_value(byte, 16)) {
        digits.push(value as u8);
        cursor += 1;
    }
    let mut saw_digit = !digits.is_empty();
    if input.get(cursor) == Some(&b'.') {
        let before = digits.len();
        let dot = cursor;
        cursor += 1;
        while let Some(value) = input.get(cursor).copied().and_then(|byte| digit_value(byte, 16)) {
            digits.push(value as u8);
            saw_digit = true;
            cursor += 1;
        }
        if !saw_digit {
            cursor = dot;
        } else {
            fraction_digits = (digits.len() - before) as i64;
        }
    }
    if !saw_digit {
        return None;
    }
    let mut exponent = 0i64;
    if matches!(input.get(cursor).copied(), Some(b'p' | b'P')) {
        let mut at = cursor + 1;
        let mut negative = false;
        match input.get(at).copied() {
            Some(b'+') => at += 1,
            Some(b'-') => {
                negative = true;
                at += 1;
            }
            _ => {}
        }
        let digits_start = at;
        let mut value: i64 = 0;
        while let Some(byte) = input.get(at).filter(|byte| byte.is_ascii_digit()) {
            value = value.saturating_mul(10).saturating_add(i64::from(*byte - b'0')).min(i64::from(EXPONENT_LIMIT));
            at += 1;
        }
        if at != digits_start {
            cursor = at;
            exponent = if negative { -value } else { value };
        }
    }
    let (kept, sticky) = significant_digits(&digits);
    if kept == 0 {
        return Some((Magnitude::Zero, cursor));
    }
    let leading = digits.iter().position(|digit| *digit != 0).unwrap_or(digits.len());
    let mantissa = Big::from_digits(&digits[leading..leading + kept], 16);
    let binary_exponent = exponent - 4 * fraction_digits;
    if binary_exponent > BINARY_RANGE {
        return Some((Magnitude::Overflow, cursor));
    }
    if binary_exponent < -BINARY_RANGE {
        return Some((Magnitude::Underflow, cursor));
    }
    let magnitude = if binary_exponent >= 0 {
        Magnitude::Finite {
            numerator: mantissa.shifted(binary_exponent as usize),
            denominator: Big::one(),
            sticky,
        }
    } else {
        Magnitude::Finite { numerator: mantissa, denominator: Big::power_of_two((-binary_exponent) as usize), sticky }
    };
    Some((magnitude, cursor))
}

fn scan(input: &[u8]) -> Option<Scan> {
    // cite: bionic strtod.c#strtod
    let mut index = 0;
    while input.get(index).copied().is_some_and(is_space) {
        index += 1;
    }
    let mut negative = false;
    match input.get(index).copied() {
        Some(b'+') => index += 1,
        Some(b'-') => {
            negative = true;
            index += 1;
        }
        _ => {}
    }
    if matches(input, index, b"inf") {
        index += 3;
        if matches(input, index, b"inity") {
            index += 5;
        }
        return Some(Scan { negative, magnitude: Magnitude::Infinite, end: index });
    }
    if matches(input, index, b"nan") {
        index += 3;
        if input.get(index) == Some(&b'(') {
            let mut at = index + 1;
            while input.get(at).is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_') {
                at += 1;
            }
            if input.get(at) == Some(&b')') {
                index = at + 1;
            }
        }
        return Some(Scan { negative, magnitude: Magnitude::Nan, end: index });
    }
    if let Some((magnitude, end)) = scan_hex(input, index) {
        return Some(Scan { negative, magnitude, end });
    }
    let (magnitude, end) = scan_decimal(input, index)?;
    Some(Scan { negative, magnitude, end })
}

fn floor_log2(numerator: &Big, denominator: &Big) -> i64 {
    let difference = numerator.bit_length() as i64 - denominator.bit_length() as i64;
    if difference >= 0 {
        if numerator.compare(&denominator.shifted(difference as usize)) == Ordering::Less {
            difference - 1
        } else {
            difference
        }
    } else if numerator.shifted((-difference) as usize).compare(denominator) == Ordering::Less {
        difference - 1
    } else {
        difference
    }
}

struct Rounding {
    value: u128,
    above_half: bool,
    exactly_half: bool,
    below_half_by_one: bool,
}

fn divide_rounded(numerator: &mut Big, denominator: &Big) -> Option<Rounding> {
    let difference = numerator.bit_length() as i64 - denominator.bit_length() as i64;
    if difference < 0 {
        return Some(Rounding { value: 0, above_half: false, exactly_half: false, below_half_by_one: false });
    }
    if difference as u32 > SIGNIFICAND_BITS {
        return None;
    }
    let mut divisor = denominator.shifted(difference as usize);
    let mut value: u128 = 0;
    for _ in 0..=difference {
        value <<= 1;
        if numerator.compare(&divisor) != Ordering::Less {
            numerator.subtract(&divisor);
            value |= 1;
        }
        divisor.shr(1);
    }
    let mut twice = numerator.shifted(1);
    let above_half = twice.compare(denominator) == Ordering::Greater;
    let exactly_half = twice.compare(denominator) == Ordering::Equal;
    twice.add_small(1);
    let below_half_by_one = twice.compare(denominator) == Ordering::Equal;
    Some(Rounding { value, above_half, exactly_half, below_half_by_one })
}

fn round_significand(numerator: &Big, denominator: &Big, sticky: bool) -> Option<u128> {
    let rounding = divide_rounded(&mut numerator.clone(), denominator)?;
    let round_up = rounding.above_half || (rounding.exactly_half && ((rounding.value & 1) != 0 || sticky)) || (rounding.below_half_by_one && sticky);
    Some(rounding.value + u128::from(round_up))
}

enum Converted {
    Bits { bits: u128, zero: bool },
    Overflow,
}

fn convert(negative: bool, numerator: &Big, denominator: &Big, sticky: bool) -> Converted {
    if numerator.is_zero() {
        return Converted::Bits { bits: u128::from(negative) << 127, zero: true };
    }
    let mut exponent = floor_log2(numerator, denominator).max(i64::from(MIN_EXPONENT));
    if exponent > i64::from(MAX_EXPONENT) {
        return Converted::Overflow;
    }
    let shift = i64::from(MANTISSA_BITS) - exponent;
    let (scaled_numerator, scaled_denominator) = if shift >= 0 {
        (numerator.shifted(shift as usize), denominator.clone())
    } else {
        (numerator.clone(), denominator.shifted((-shift) as usize))
    };
    let Some(mut significand) = round_significand(&scaled_numerator, &scaled_denominator, sticky) else {
        return Converted::Overflow;
    };
    if significand == 0 {
        return Converted::Bits { bits: u128::from(negative) << 127, zero: true };
    }
    while significand >= 1u128 << SIGNIFICAND_BITS {
        significand >>= 1;
        exponent += 1;
    }
    if exponent > i64::from(MAX_EXPONENT) {
        return Converted::Overflow;
    }
    let sign = u128::from(negative) << 127;
    if significand >= 1u128 << MANTISSA_BITS {
        let field = ((exponent as i32 + EXPONENT_BIAS) as u128) << MANTISSA_BITS;
        Converted::Bits { bits: sign | field | (significand & ((1u128 << MANTISSA_BITS) - 1)), zero: false }
    } else {
        Converted::Bits { bits: sign | significand, zero: false }
    }
}

fn evaluate(scan: &Scan) -> (u128, Option<i32>) {
    let sign = u128::from(scan.negative) << 127;
    match &scan.magnitude {
        Magnitude::Zero => (sign, None),
        Magnitude::Infinite => (sign | INFINITY_BITS, None),
        Magnitude::Overflow => (sign | INFINITY_BITS, Some(ERANGE)),
        Magnitude::Underflow => (sign, Some(ERANGE)),
        Magnitude::Nan => (sign | QUIET_NAN, None),
        Magnitude::Finite { numerator, denominator, sticky } => match convert(scan.negative, numerator, denominator, *sticky) {
            Converted::Bits { bits, zero } => (bits, (zero && !numerator.is_zero()).then_some(ERANGE)),
            Converted::Overflow => (sign | INFINITY_BITS, Some(ERANGE)),
        },
    }
}

fn convert_bytes(input: &[u8]) -> (u128, usize, Option<i32>) {
    match scan(input) {
        None => (0, 0, None),
        Some(found) => {
            let (bits, errno) = evaluate(&found);
            (bits, found.end, errno)
        }
    }
}

fn strtold(context: &mut CallContext) {
    // cite: bionic strtold.cpp#strtold
    let start = context.arg_ptr(0);
    let (bits, consumed, errno) = convert_bytes(cstr_bytes(start));
    let end = context.arg_ptr(1);
    if end != 0 {
        write_value::<usize>(end, if consumed == 0 { start } else { start + consumed });
    }
    if let Some(code) = errno {
        context.set_errno(code);
    }
    context.ret_quad(bits);
}

fn wide_space(character: u32) -> bool {
    // cite: bionic wctype.cpp#iswspace
    matches!(character, 0x20 | 0x09 | 0x0a | 0x0b | 0x0c | 0x0d)
}

fn wcstold(context: &mut CallContext) {
    // cite: bionic wcstod.cpp#wcstold
    let original = context.arg_ptr(0);
    let mut cursor = original;
    while wide_space(read_value::<u32>(cursor)) {
        cursor += 4;
    }
    let mut span = 0usize;
    loop {
        let character = read_value::<u32>(cursor + span * 4);
        if character >= 0x80 || !WIDE_NUMERIC.contains(&(character as u8)) {
            break;
        }
        span += 1;
    }
    let ascii: Vec<u8> = (0..span).map(|index| read_value::<u32>(cursor + index * 4) as u8).collect();
    let (bits, consumed, errno) = convert_bytes(&ascii);
    let end = context.arg_ptr(1);
    if end != 0 {
        write_value::<usize>(end, if consumed == 0 { original } else { cursor + consumed * 4 });
    }
    if let Some(code) = errno {
        context.set_errno(code);
    }
    context.ret_quad(bits);
}

pub fn register() {
    in_libc("strtold", strtold);
    in_libc("strtold_l", strtold);
    in_libc("wcstold", wcstold);
    in_libc("wcstold_l", wcstold);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bits_of(text: &str) -> u128 {
        convert_bytes(text.as_bytes()).0
    }

    #[test]
    fn correctly_rounded_binary128_vectors_match_exact_rational_arithmetic_oracles() {
        let vectors: [(&str, u128); 9] = [
            ("0.1", 0x3ffb_9999_9999_9999_9999_9999_9999_999a),
            ("1.5", 0x3fff_8000_0000_0000_0000_0000_0000_0000),
            ("3.14159265358979323846264338327950288", 0x4000_921f_b544_42d1_8469_898c_c517_01b8),
            ("123456789012345678901234567890.5", 0x405f_8ee9_0ff6_c373_e0ee_4e3f_0ad2_8000),
            ("2.2250738585072014e-308", 0x3c01_0000_0000_0000_008c_304c_cf86_7de0),
            ("1e400", 0x452f_b4ec_7f91_973f_f3cb_1ccf_26fb_c178),
            ("1e-4000", 0x0c17_387a_e70c_9e70_0b80_4973_2d11_a23d),
            ("1e-4950", 0x0000_0000_0000_0000_0005_7c96_47e1_a018),
            ("1e4932", 0x7ffe_ae59_6552_b8fd_ed99_d037_e3d0_4b75),
        ];
        for (text, expected) in vectors {
            assert_eq!(bits_of(text), expected, "{text}");
        }
    }

    #[test]
    fn hexadecimal_vectors_are_exact_and_the_minimum_and_maximum_finite_values_round_to_the_extremes() {
        assert_eq!(bits_of("0x1.8p3"), 0x4002_8000_0000_0000_0000_0000_0000_0000);
        assert_eq!(bits_of("0x1p-16494"), 0x0000_0000_0000_0000_0000_0000_0000_0001);
        assert_eq!(bits_of("0x1.ffffffffffffffffffffffffffffp16383"), 0x7ffe_ffff_ffff_ffff_ffff_ffff_ffff_ffff);
    }

    #[test]
    fn overflow_and_underflow_set_erange_and_saturate_to_infinity_or_zero() {
        assert_eq!(evaluate(&scan(b"1e4933").unwrap()), (INFINITY_BITS, Some(ERANGE)));
        assert_eq!(evaluate(&scan(b"1e-5000").unwrap()), (0, Some(ERANGE)));
        assert_eq!(evaluate(&scan(b"1e-4951").unwrap()), (0x8c75_6d96_9002, None), "the smallest subnormals are still representable");
    }

    #[test]
    fn nothing_converted_reports_no_consumption_and_no_errno() {
        for input in ["", ".", "abc", "  "] {
            assert_eq!(convert_bytes(input.as_bytes()), (0, 0, None), "{input:?}");
        }
    }

    #[test]
    fn signed_forms_and_specials_follow_the_gdtoa_scanner() {
        assert_eq!(convert_bytes(b"-0").0, 1u128 << 127);
        assert_eq!(convert_bytes(b"-0").2, None);
        assert_eq!(bits_of("nan"), QUIET_NAN);
        assert_eq!(bits_of("-nan"), QUIET_NAN | 1u128 << 127);
        assert_eq!(bits_of("infinity"), INFINITY_BITS);
        assert_eq!(convert_bytes(b"0.1f").1, 3);
        assert_eq!(convert_bytes(b"  1.5xyz").1, 5);
        assert_eq!(convert_bytes(b"1e").1, 1);
        assert_eq!(convert_bytes(b"0x").1, 1);
        assert_eq!(convert_bytes(b"1e5").0, bits_of("100000"));
    }

    #[test]
    fn half_way_values_round_to_even_and_a_sticky_digit_breaks_the_tie_upward() {
        assert_eq!(bits_of("0x1.00000000000000000000000000008p0"), 0x3fff_0000_0000_0000_0000_0000_0000_0000);
        assert_eq!(bits_of("0x1.00000000000000000000000000009p0"), 0x3fff_0000_0000_0000_0000_0000_0000_0001);
        assert_eq!(
            bits_of("0x1.00000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001p0"),
            0x3fff_0000_0000_0000_0000_0000_0000_0001
        );
    }

    #[test]
    fn decimal_exponents_outside_the_binary128_range_saturate_instead_of_building_huge_integers() {
        assert_eq!(evaluate(&scan(b"1e-400").unwrap()), (0x3ace_2bfc_fc0f_923d_f5f4_7263_70a1_be12, None));
        assert_eq!(evaluate(&scan(b"1e-6000").unwrap()), (0, Some(ERANGE)));
        assert_eq!(evaluate(&scan(b"1e6000").unwrap()), (INFINITY_BITS, Some(ERANGE)));
    }

    #[test]
    fn a_huge_number_of_fraction_digits_still_lands_on_the_representable_neighbour() {
        let text = format!("1.{}1", "0".repeat(400));
        assert_eq!(bits_of(&text), 0x3fff_0000_0000_0000_0000_0000_0000_0000);
        assert_eq!(evaluate(&scan(b"1e-4000").unwrap()), (0x0c17_387a_e70c_9e70_0b80_4973_2d11_a23d, None));
    }
}
