use std::collections::HashSet;

use super::*;

impl DexProgram {
    pub(super) fn initialize_big_integer_from_string(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }
        let DexValue::String(value) = &invocation.arguments[1] else {
            return None;
        };
        if !is_supported_big_integer_literal(value) {
            return None;
        }

        let key = map_identity_key(&invocation.arguments[0]);
        self.runtime
            .borrow_mut()
            .big_integer_values
            .insert(key, normalize_big_integer_literal(value));

        Some(())
    }

    pub(super) fn big_integer_to_string(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.big_integer_value(&invocation.arguments[0])
    }

    pub(super) fn big_integer_value_of_long(&self, invocation: &DexInvocation) -> Option<DexValue> {
        let value = match invocation.arguments.as_slice() {
            [DexValue::Long(value)] | [DexValue::Long(value), DexValue::Long(_)] => *value,
            _ => return None,
        };

        let result = self.allocate_object("Ljava/math/BigInteger;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_integer_values
            .insert(key, value.to_string());

        Some(result)
    }

    pub(super) fn big_integer_multiply(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.big_integer_value(&invocation.arguments[0])?;
        let right = self.big_integer_value(&invocation.arguments[1])?;
        let product = multiply_big_integer_strings_exact(&left, &right)?;

        let result = self.allocate_object("Ljava/math/BigInteger;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_integer_values
            .insert(key, product);

        Some(result)
    }

    pub(super) fn big_integer_value(&self, value: &DexValue) -> Option<String> {
        match value {
            DexValue::Object(_) | DexValue::Reference(_) => {
                let key = map_identity_key(value);
                if let Some(value) = self.runtime.borrow().big_integer_values.get(&key) {
                    return Some(value.clone());
                }
            }
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        let DexValue::Reference(reference) = value else {
            return None;
        };

        reference
            .strip_prefix("Ljava/math/BigInteger;:")
            .map(str::to_owned)
    }

    pub(super) fn initialize_math_context(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }
        let DexValue::Int(precision) = invocation.arguments[1] else {
            return None;
        };
        let rounding_mode = self.enum_name_value(&invocation.arguments[2])?;

        let key = map_identity_key(&invocation.arguments[0]);
        self.runtime.borrow_mut().math_context_values.insert(
            key,
            MathContextRuntimeValue {
                precision,
                rounding_mode,
            },
        );

        Some(())
    }

    pub(super) fn initialize_big_decimal_from_string(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        let DexValue::String(value) = &invocation.arguments[1] else {
            return None;
        };
        if !is_supported_big_decimal_literal(value) {
            return None;
        }

        let key = map_identity_key(&invocation.arguments[0]);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, value.clone().to_string());

        Some(())
    }

    pub(super) fn initialize_big_decimal_from_big_integer(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }
        let value = self.big_integer_value(&invocation.arguments[1])?;

        let key = map_identity_key(&invocation.arguments[0]);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, value);

        Some(())
    }

    pub(super) fn initialize_big_decimal_from_int(&self, invocation: &DexInvocation) -> Option<()> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        match &invocation.arguments[0] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }
        let DexValue::Int(value) = invocation.arguments[1] else {
            return None;
        };

        let key = map_identity_key(&invocation.arguments[0]);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, value.to_string());

        Some(())
    }

    pub(super) fn initialize_big_decimal_from_double(
        &self,
        invocation: &DexInvocation,
    ) -> Option<()> {
        let (receiver, bits) = match invocation.arguments.as_slice() {
            [receiver, DexValue::Long(bits)]
            | [receiver, DexValue::Long(bits), DexValue::Long(_)] => (receiver, *bits),
            _ => return None,
        };

        match receiver {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };
        let value = exact_f64_decimal_string(f64::from_bits(bits as u64))?;

        let key = map_identity_key(receiver);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, value);

        Some(())
    }

    pub(super) fn big_decimal_value_of_double(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexValue> {
        let bits = match invocation.arguments.as_slice() {
            [DexValue::Long(bits)] | [DexValue::Long(bits), DexValue::Long(_)] => *bits,
            _ => return None,
        };
        // cite: libcore BigDecimal.java#valueOf, BigDecimal
        let value =
            java_scientific_to_plain_decimal(&java_double_to_string(f64::from_bits(bits as u64))?)?;

        let result = self.allocate_object("Ljava/math/BigDecimal;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, value);

        Some(result)
    }

    pub(super) fn big_decimal_to_string(&self, invocation: &DexInvocation) -> Option<String> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.big_decimal_value(&invocation.arguments[0])
    }

    pub(super) fn big_decimal_float_value(&self, invocation: &DexInvocation) -> Option<f32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        self.big_decimal_value(&invocation.arguments[0])?
            .parse::<f32>()
            .ok()
    }

    pub(super) fn big_decimal_double_value(&self, invocation: &DexInvocation) -> Option<i64> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let value = self
            .big_decimal_value(&invocation.arguments[0])?
            .parse::<f64>()
            .ok()?;

        Some(value.to_bits() as i64)
    }

    pub(super) fn big_decimal_int_value(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 1 {
            return None;
        }

        let value = self.big_decimal_value(&invocation.arguments[0])?;

        big_decimal_int_value_exact(&value)
    }

    pub(super) fn big_decimal_value(&self, value: &DexValue) -> Option<String> {
        match value {
            DexValue::Object(_) | DexValue::Reference(_) => {
                let key = map_identity_key(value);
                if let Some(value) = self.runtime.borrow().big_decimal_values.get(&key) {
                    return Some(value.clone());
                }
            }
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        }

        let DexValue::Reference(reference) = value else {
            return None;
        };

        reference
            .strip_prefix("Ljava/math/BigDecimal;:")
            .map(str::to_owned)
    }

    pub(super) fn big_decimal_set_scale(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let value = self.big_decimal_value(&invocation.arguments[0])?;
        let DexValue::Int(scale) = invocation.arguments[1] else {
            return None;
        };
        match &invocation.arguments[2] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };
        let rounding_mode = self.enum_name_value(&invocation.arguments[2])?;

        let scaled = set_big_decimal_scale(&value, scale, &rounding_mode)?;
        let result = self.allocate_object("Ljava/math/BigDecimal;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, scaled);

        Some(result)
    }

    pub(super) fn big_decimal_add(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.big_decimal_value(&invocation.arguments[0])?;
        let right = self.big_decimal_value(&invocation.arguments[1])?;
        let sum = add_big_decimal_strings_exact(&left, &right)?;

        let result = self.allocate_object("Ljava/math/BigDecimal;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, sum);

        Some(result)
    }

    pub(super) fn big_decimal_subtract(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.big_decimal_value(&invocation.arguments[0])?;
        let right = self.big_decimal_value(&invocation.arguments[1])?;
        let difference = subtract_big_decimal_strings_exact(&left, &right)?;

        let result = self.allocate_object("Ljava/math/BigDecimal;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, difference);

        Some(result)
    }

    pub(super) fn big_decimal_multiply(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.big_decimal_value(&invocation.arguments[0])?;
        let right = self.big_decimal_value(&invocation.arguments[1])?;
        let product = multiply_big_decimal_strings_exact(&left, &right)?;

        let result = self.allocate_object("Ljava/math/BigDecimal;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, product);

        Some(result)
    }

    pub(super) fn big_decimal_divide(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.big_decimal_value(&invocation.arguments[0])?;
        let right = self.big_decimal_value(&invocation.arguments[1])?;
        let quotient = divide_big_decimal_strings_exact(&left, &right)?;

        let result = self.allocate_object("Ljava/math/BigDecimal;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, quotient);

        Some(result)
    }

    pub(super) fn big_decimal_sqrt(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let value = self.big_decimal_value(&invocation.arguments[0])?;
        let context_key = map_identity_key(&invocation.arguments[1]);
        let context = self
            .runtime
            .borrow()
            .math_context_values
            .get(&context_key)
            .cloned()?;
        let square_root = sqrt_big_decimal_string(&value, &context)?;

        let result = self.allocate_object("Ljava/math/BigDecimal;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, square_root);

        Some(result)
    }

    pub(super) fn big_decimal_pow(&self, invocation: &DexInvocation) -> Option<DexValue> {
        if invocation.arguments.len() != 3 {
            return None;
        }

        let value = self.big_decimal_value(&invocation.arguments[0])?;
        let DexValue::Int(exponent) = invocation.arguments[1] else {
            return None;
        };
        match &invocation.arguments[2] {
            DexValue::Object(_) | DexValue::Reference(_) => {}
            DexValue::Uninitialized
            | DexValue::Null
            | DexValue::Int(_)
            | DexValue::Long(_)
            | DexValue::Float(_)
            | DexValue::String(_) => return None,
        };

        let power = pow_big_decimal_strings_exact(&value, exponent)?;

        let result = self.allocate_object("Ljava/math/BigDecimal;".to_owned());
        let key = map_identity_key(&result);
        self.runtime
            .borrow_mut()
            .big_decimal_values
            .insert(key, power);

        Some(result)
    }

    pub(super) fn big_decimal_compare_to(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = self.big_decimal_value(&invocation.arguments[0])?;
        let right = self.big_decimal_value(&invocation.arguments[1])?;

        compare_big_decimal_strings_exact(&left, &right)
    }
}

pub(super) fn is_supported_big_decimal_literal(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let mut index = 0usize;
    if matches!(bytes[index], b'+' | b'-') {
        index += 1;
        if index == bytes.len() {
            return false;
        }
    }

    let mut digit_count = 0usize;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        digit_count += 1;
        index += 1;
    }

    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            digit_count += 1;
            index += 1;
        }
    }

    if digit_count == 0 {
        return false;
    }

    if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
        index += 1;
        if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }

        let exponent_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == exponent_start {
            return false;
        }
    }

    index == bytes.len()
}

pub(super) fn is_supported_big_integer_literal(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let mut index = 0usize;
    if matches!(bytes[index], b'+' | b'-') {
        index += 1;
        if index == bytes.len() {
            return false;
        }
    }

    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            return false;
        }
        index += 1;
    }

    true
}

pub(super) fn normalize_big_integer_literal(value: &str) -> String {
    let (negative, unsigned) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = value.strip_prefix('+') {
        (false, rest)
    } else {
        (false, value)
    };

    let unsigned = unsigned.trim_start_matches('0');
    if unsigned.is_empty() {
        return "0".to_owned();
    }
    if negative {
        return format!("-{unsigned}");
    }

    unsigned.to_owned()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ParsedBigDecimal {
    negative: bool,
    digits: Vec<u8>,
    scale: usize,
}

pub(super) fn add_big_decimal_strings_exact(left: &str, right: &str) -> Option<String> {
    let left = parse_plain_big_decimal(left)?;
    let right = parse_plain_big_decimal(right)?;
    let scale = left.scale.max(right.scale);

    let left_digits = scale_big_decimal_digits(&left, scale)?;
    let right_digits = scale_big_decimal_digits(&right, scale)?;

    let (negative, mut digits) = if left.negative == right.negative {
        (
            left.negative,
            add_decimal_digits(&left_digits, &right_digits),
        )
    } else {
        match compare_decimal_digits(&left_digits, &right_digits) {
            std::cmp::Ordering::Greater => (
                left.negative,
                subtract_decimal_digits(&left_digits, &right_digits)?,
            ),
            std::cmp::Ordering::Less => (
                right.negative,
                subtract_decimal_digits(&right_digits, &left_digits)?,
            ),
            std::cmp::Ordering::Equal => (false, vec![0]),
        }
    };

    trim_decimal_digits(&mut digits);
    let negative = negative && !is_zero_decimal_digits(&digits);

    format_plain_big_decimal(negative, &digits, scale)
}

pub(super) fn subtract_big_decimal_strings_exact(left: &str, right: &str) -> Option<String> {
    let mut right = parse_plain_big_decimal(right)?;
    right.negative = !right.negative && !is_zero_decimal_digits(&right.digits);
    let right = format_plain_big_decimal(right.negative, &right.digits, right.scale)?;

    add_big_decimal_strings_exact(left, &right)
}

pub(super) fn multiply_big_decimal_strings_exact(left: &str, right: &str) -> Option<String> {
    let left = parse_plain_big_decimal(left)?;
    let right = parse_plain_big_decimal(right)?;
    let scale = left.scale.checked_add(right.scale)?;

    let mut digits = multiply_decimal_digits(&left.digits, &right.digits)?;
    trim_decimal_digits(&mut digits);
    let negative = left.negative ^ right.negative;
    let negative = negative && !is_zero_decimal_digits(&digits);

    format_plain_big_decimal(negative, &digits, scale)
}

pub(super) fn divide_big_decimal_strings_exact(left: &str, right: &str) -> Option<String> {
    let left = parse_plain_big_decimal(left)?;
    let right = parse_plain_big_decimal(right)?;
    if is_zero_decimal_digits(&right.digits) {
        return None;
    }

    let preferred_scale = left.scale as isize - right.scale as isize;
    let mut scale = usize::try_from(preferred_scale.max(0)).ok()?;
    let initial_power = right.scale as isize - left.scale as isize + scale as isize;
    let initial_power = usize::try_from(initial_power).ok()?;
    let mut numerator = left.digits.clone();
    numerator.extend(std::iter::repeat(0).take(initial_power));

    let mut seen_remainders = HashSet::new();
    loop {
        let (mut quotient, remainder) = divide_decimal_digits(&numerator, &right.digits)?;
        if is_zero_decimal_digits(&remainder) {
            trim_decimal_digits(&mut quotient);
            let negative = (left.negative ^ right.negative) && !is_zero_decimal_digits(&quotient);
            return format_plain_big_decimal(negative, &quotient, scale);
        }

        let remainder_key = decimal_digits_key(&remainder);
        if !seen_remainders.insert(remainder_key) {
            return None;
        }

        scale = scale.checked_add(1)?;
        numerator.push(0);
    }
}

pub(super) fn pow_big_decimal_strings_exact(value: &str, exponent: i32) -> Option<String> {
    let mut exponent = usize::try_from(exponent).ok()?;
    let mut base = value.to_owned();
    let mut result = "1".to_owned();

    while exponent > 0 {
        if exponent & 1 == 1 {
            result = multiply_big_decimal_strings_exact(&result, &base)?;
        }
        exponent >>= 1;
        if exponent > 0 {
            base = multiply_big_decimal_strings_exact(&base, &base)?;
        }
    }

    Some(result)
}

pub(super) fn multiply_big_integer_strings_exact(left: &str, right: &str) -> Option<String> {
    let left = parse_plain_big_decimal(left)?;
    let right = parse_plain_big_decimal(right)?;
    if left.scale != 0 || right.scale != 0 {
        return None;
    }

    let mut digits = multiply_decimal_digits(&left.digits, &right.digits)?;
    trim_decimal_digits(&mut digits);
    let negative = (left.negative ^ right.negative) && !is_zero_decimal_digits(&digits);

    format_plain_big_decimal(negative, &digits, 0)
}

pub(super) fn compare_big_decimal_strings_exact(left: &str, right: &str) -> Option<i32> {
    let left = parse_plain_big_decimal(left)?;
    let right = parse_plain_big_decimal(right)?;

    if left.negative != right.negative {
        return if left.negative { Some(-1) } else { Some(1) };
    }

    let scale = left.scale.max(right.scale);
    let left_digits = scale_big_decimal_digits(&left, scale)?;
    let right_digits = scale_big_decimal_digits(&right, scale)?;
    let ordering = compare_decimal_digits(&left_digits, &right_digits);
    let value = match ordering {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    };

    if left.negative {
        return Some(-value);
    }

    Some(value)
}

pub(super) fn big_decimal_int_value_exact(value: &str) -> Option<i32> {
    let parsed = parse_plain_big_decimal(value)?;
    let integer_digit_count = parsed.digits.len().saturating_sub(parsed.scale);
    let integer_digits = if integer_digit_count == 0 {
        &[0][..]
    } else {
        &parsed.digits[..integer_digit_count]
    };

    let mut result = 0i32;
    for digit in integer_digits {
        result = result.wrapping_mul(10).wrapping_add(i32::from(*digit));
    }
    if parsed.negative {
        result = result.wrapping_neg();
    }

    Some(result)
}

pub(super) fn exact_f64_decimal_string(value: f64) -> Option<String> {
    let bits = value.to_bits();
    let negative = bits >> 63 != 0;
    let exponent_bits = ((bits >> 52) & 0x07ff) as i32;
    let fraction_bits = bits & 0x000f_ffff_ffff_ffff;

    if exponent_bits == 0x07ff {
        return None;
    }
    if exponent_bits == 0 && fraction_bits == 0 {
        return Some("0".to_owned());
    }

    let (significand, binary_exponent) = if exponent_bits == 0 {
        (fraction_bits, -1074)
    } else {
        ((1u64 << 52) | fraction_bits, exponent_bits - 1075)
    };
    let mut digits = decimal_digits_from_u64(significand)?;
    let scale = if binary_exponent >= 0 {
        for _ in 0..usize::try_from(binary_exponent).ok()? {
            digits = decimal_digits_mul_small(&digits, 2)?;
        }
        0usize
    } else {
        let scale = usize::try_from(-binary_exponent).ok()?;
        for _ in 0..scale {
            digits = decimal_digits_mul_small(&digits, 5)?;
        }
        scale
    };

    let value = format_plain_big_decimal(negative, &digits, scale)?;
    strip_trailing_fraction_zeros(value)
}

// cite: libcore Double.java#toString
pub(super) fn java_double_to_string(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }

    Some(java_floating_text(
        &format!("{value:e}"),
        value.abs() >= 1.0e-3 && value.abs() < 1.0e7,
        value == 0.0,
    ))
}

// cite: libcore BigDecimal.java#BigDecimal, layoutChars
fn java_scientific_to_plain_decimal(text: &str) -> Option<String> {
    let Some((mantissa, exponent)) = text.split_once('E') else {
        return Some(text.to_owned());
    };
    let exponent: i64 = exponent.parse().ok()?;
    let (negative, mantissa) = match mantissa.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, mantissa),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits: String = integer.chars().chain(fraction.chars()).collect();
    let scale = i64::try_from(fraction.len()).ok()?.checked_sub(exponent)?;
    if i64::try_from(digits.len()).ok()? - 1 - scale < -6 {
        return None;
    }
    let scale = usize::try_from(scale).ok()?;
    let digits = format!("{digits:0>width$}", width = scale + 1);
    let split = digits.len() - scale;
    let sign = if negative { "-" } else { "" };
    if scale == 0 {
        return Some(format!("{sign}{digits}"));
    }

    Some(format!("{sign}{}.{}", &digits[..split], &digits[split..]))
}

pub(super) fn decimal_digits_from_u64(value: u64) -> Option<Vec<u8>> {
    let mut digits = Vec::new();
    for byte in value.to_string().bytes() {
        if !byte.is_ascii_digit() {
            return None;
        }
        digits.push(byte - b'0');
    }
    trim_decimal_digits(&mut digits);

    Some(digits)
}

pub(super) fn sqrt_big_decimal_string(
    value: &str,
    context: &MathContextRuntimeValue,
) -> Option<String> {
    let parsed = parse_plain_big_decimal(value)?;
    if parsed.negative {
        return None;
    }
    if is_zero_decimal_digits(&parsed.digits) {
        return Some("0".to_owned());
    }

    let precision = usize::try_from(context.precision).ok()?;
    if precision == 0 {
        return sqrt_big_decimal_exact(&parsed);
    }

    let output_scale = sqrt_output_scale(&parsed, precision)?;
    let working_scale = output_scale
        .checked_add(1)?
        .max(parsed.scale.checked_add(1)? / 2);
    let scaled_power = working_scale.checked_mul(2)?.checked_sub(parsed.scale)?;
    let mut radicand = parsed.digits.clone();
    radicand.extend(std::iter::repeat(0).take(scaled_power));
    trim_decimal_digits(&mut radicand);

    let (root_digits, root_remainder) = integer_sqrt_decimal_digits(&radicand)?;
    let rounded_digits = round_scaled_decimal_digits(
        &root_digits,
        working_scale,
        output_scale,
        &context.rounding_mode,
        false,
        !is_zero_decimal_digits(&root_remainder),
    )?;
    let value = format_plain_big_decimal(false, &rounded_digits, output_scale)?;

    strip_trailing_fraction_zeros(value)
}

pub(super) fn sqrt_big_decimal_exact(value: &ParsedBigDecimal) -> Option<String> {
    let output_scale = value.scale.checked_add(1)? / 2;
    let scaled_power = output_scale.checked_mul(2)?.checked_sub(value.scale)?;
    let mut radicand = value.digits.clone();
    radicand.extend(std::iter::repeat(0).take(scaled_power));
    trim_decimal_digits(&mut radicand);

    let (root_digits, root_remainder) = integer_sqrt_decimal_digits(&radicand)?;
    if !is_zero_decimal_digits(&root_remainder) {
        return None;
    }

    let value = format_plain_big_decimal(false, &root_digits, output_scale)?;
    strip_trailing_fraction_zeros(value)
}

pub(super) fn sqrt_output_scale(value: &ParsedBigDecimal, precision: usize) -> Option<usize> {
    if precision == 0 {
        return None;
    }

    let adjusted_exponent =
        isize::try_from(value.digits.len()).ok()? - isize::try_from(value.scale).ok()? - 1;
    let root_adjusted_exponent = floor_divide_by_two(adjusted_exponent);

    if root_adjusted_exponent >= 0 {
        let integer_digits = usize::try_from(root_adjusted_exponent)
            .ok()?
            .checked_add(1)?;
        return Some(precision.saturating_sub(integer_digits));
    }

    let leading_fraction_zeros = usize::try_from(-root_adjusted_exponent - 1).ok()?;
    precision.checked_add(leading_fraction_zeros)
}

pub(super) fn floor_divide_by_two(value: isize) -> isize {
    if value >= 0 {
        return value / 2;
    }

    -((-value + 1) / 2)
}

pub(super) fn integer_sqrt_decimal_digits(digits: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if digits.is_empty() {
        return None;
    }

    let mut groups = Vec::new();
    let mut index = 0usize;
    if digits.len() % 2 == 1 {
        groups.push(digits[0]);
        index = 1;
    }
    while index < digits.len() {
        let group = digits[index]
            .checked_mul(10)?
            .checked_add(digits[index + 1])?;
        groups.push(group);
        index += 2;
    }

    let mut root = vec![0];
    let mut remainder = vec![0];
    for group in groups {
        remainder = decimal_digits_shift_base100_add(&remainder, group)?;

        let mut selected_digit = 0u8;
        let mut selected_value = vec![0];
        for digit in 0..=9 {
            let trial = decimal_digits_add_small(&decimal_digits_mul_small(&root, 20)?, digit)?;
            let trial = decimal_digits_mul_small(&trial, u16::from(digit))?;
            if compare_decimal_digits(&trial, &remainder) == std::cmp::Ordering::Greater {
                break;
            }
            selected_digit = digit;
            selected_value = trial;
        }

        remainder = subtract_decimal_digits(&remainder, &selected_value)?;
        root.push(selected_digit);
        trim_decimal_digits(&mut root);
    }

    trim_decimal_digits(&mut root);
    trim_decimal_digits(&mut remainder);
    Some((root, remainder))
}

pub(super) fn decimal_digits_shift_base100_add(digits: &[u8], group: u8) -> Option<Vec<u8>> {
    if group > 99 {
        return None;
    }

    let mut value = if is_zero_decimal_digits(digits) {
        Vec::new()
    } else {
        digits.to_vec()
    };
    value.push(group / 10);
    value.push(group % 10);
    trim_decimal_digits(&mut value);

    Some(value)
}

pub(super) fn decimal_digits_mul_small(digits: &[u8], multiplier: u16) -> Option<Vec<u8>> {
    if digits.is_empty() {
        return None;
    }
    if multiplier == 0 || is_zero_decimal_digits(digits) {
        return Some(vec![0]);
    }

    let mut result = Vec::with_capacity(digits.len() + 3);
    let mut carry = 0u16;
    for digit in digits.iter().rev() {
        let value = u16::from(*digit)
            .checked_mul(multiplier)?
            .checked_add(carry)?;
        result.push(u8::try_from(value % 10).ok()?);
        carry = value / 10;
    }
    while carry > 0 {
        result.push(u8::try_from(carry % 10).ok()?);
        carry /= 10;
    }

    result.reverse();
    trim_decimal_digits(&mut result);
    Some(result)
}

pub(super) fn decimal_digits_add_small(digits: &[u8], addend: u8) -> Option<Vec<u8>> {
    if digits.is_empty() {
        return None;
    }

    let mut result = digits.to_vec();
    let mut carry = u16::from(addend);
    let mut index = result.len();
    while carry > 0 {
        if index == 0 {
            result.insert(0, u8::try_from(carry % 10).ok()?);
            carry /= 10;
            continue;
        }

        index -= 1;
        let value = u16::from(result[index]).checked_add(carry)?;
        result[index] = u8::try_from(value % 10).ok()?;
        carry = value / 10;
    }

    trim_decimal_digits(&mut result);
    Some(result)
}

pub(super) fn round_scaled_decimal_digits(
    digits: &[u8],
    input_scale: usize,
    output_scale: usize,
    rounding_mode: &str,
    negative: bool,
    extra_sticky: bool,
) -> Option<Vec<u8>> {
    if output_scale > input_scale {
        let mut value = digits.to_vec();
        value.extend(std::iter::repeat(0).take(output_scale - input_scale));
        trim_decimal_digits(&mut value);
        return Some(value);
    }

    let scale_delta = input_scale - output_scale;
    let (mut quotient, removed) = split_decimal_digits_for_scale_rounding(digits, scale_delta)?;
    if scale_delta == 0 {
        return Some(quotient);
    }

    let guard = removed[0];
    let sticky = extra_sticky || removed[1..].iter().any(|digit| *digit != 0);
    let removed_nonzero = guard != 0 || sticky;
    let last_digit_odd = quotient.last().map(|digit| digit % 2 == 1).unwrap_or(false);
    let increment = match rounding_mode {
        "UP" => removed_nonzero,
        "DOWN" => false,
        "CEILING" => !negative && removed_nonzero,
        "FLOOR" => negative && removed_nonzero,
        "HALF_UP" => guard > 5 || guard == 5,
        "HALF_DOWN" => guard > 5 || (guard == 5 && sticky),
        "HALF_EVEN" => guard > 5 || (guard == 5 && (sticky || last_digit_odd)),
        "UNNECESSARY" => {
            if removed_nonzero {
                return None;
            }
            false
        }
        _ => return None,
    };

    if increment {
        quotient = decimal_digits_add_small(&quotient, 1)?;
    }
    trim_decimal_digits(&mut quotient);

    Some(quotient)
}

pub(super) fn split_decimal_digits_for_scale_rounding(
    digits: &[u8],
    scale_delta: usize,
) -> Option<(Vec<u8>, Vec<u8>)> {
    if digits.is_empty() {
        return None;
    }
    if scale_delta == 0 {
        return Some((digits.to_vec(), Vec::new()));
    }

    if digits.len() > scale_delta {
        let split = digits.len() - scale_delta;
        return Some((digits[..split].to_vec(), digits[split..].to_vec()));
    }

    let mut removed = vec![0; scale_delta - digits.len()];
    removed.extend_from_slice(digits);

    Some((vec![0], removed))
}

pub(super) fn strip_trailing_fraction_zeros(mut value: String) -> Option<String> {
    if !value.contains('.') {
        return Some(value);
    }

    while value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    if value.is_empty() || value == "-" {
        return Some("0".to_owned());
    }

    Some(value)
}

pub(super) fn parse_plain_big_decimal(value: &str) -> Option<ParsedBigDecimal> {
    if value.contains('e') || value.contains('E') {
        return None;
    }

    let (negative, unsigned) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = value.strip_prefix('+') {
        (false, rest)
    } else {
        (false, value)
    };

    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if integer.is_empty() && fraction.is_empty() {
        return None;
    }
    if !integer.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let mut digits = Vec::new();
    for byte in integer.bytes().chain(fraction.bytes()) {
        digits.push(byte - b'0');
    }
    if digits.is_empty() {
        return None;
    }

    trim_decimal_digits(&mut digits);
    let negative = negative && !is_zero_decimal_digits(&digits);

    Some(ParsedBigDecimal {
        negative,
        digits,
        scale: fraction.len(),
    })
}

pub(super) fn scale_big_decimal_digits(value: &ParsedBigDecimal, scale: usize) -> Option<Vec<u8>> {
    if scale < value.scale {
        return None;
    }

    let mut digits = value.digits.clone();
    digits.extend(std::iter::repeat(0).take(scale - value.scale));
    trim_decimal_digits(&mut digits);

    Some(digits)
}

pub(super) fn add_decimal_digits(left: &[u8], right: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let mut carry = 0u8;
    let mut left_index = left.len();
    let mut right_index = right.len();

    while left_index > 0 || right_index > 0 || carry != 0 {
        let mut value = carry;
        if left_index > 0 {
            left_index -= 1;
            value += left[left_index];
        }
        if right_index > 0 {
            right_index -= 1;
            value += right[right_index];
        }

        result.push(value % 10);
        carry = value / 10;
    }

    result.reverse();
    result
}

pub(super) fn subtract_decimal_digits(left: &[u8], right: &[u8]) -> Option<Vec<u8>> {
    if compare_decimal_digits(left, right) == std::cmp::Ordering::Less {
        return None;
    }

    let mut result = Vec::new();
    let mut borrow = 0i16;
    let mut left_index = left.len();
    let mut right_index = right.len();

    while left_index > 0 {
        left_index -= 1;
        let mut value = i16::from(left[left_index]) - borrow;

        if right_index > 0 {
            right_index -= 1;
            value -= i16::from(right[right_index]);
        }

        if value < 0 {
            value += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }

        result.push(u8::try_from(value).ok()?);
    }

    if borrow != 0 {
        return None;
    }

    result.reverse();
    trim_decimal_digits(&mut result);
    Some(result)
}

pub(super) fn multiply_decimal_digits(left: &[u8], right: &[u8]) -> Option<Vec<u8>> {
    if left.is_empty() || right.is_empty() {
        return None;
    }

    if is_zero_decimal_digits(left) || is_zero_decimal_digits(right) {
        return Some(vec![0]);
    }

    let mut accumulators = vec![0u16; left.len().checked_add(right.len())?];
    for (left_offset, left_digit) in left.iter().rev().enumerate() {
        for (right_offset, right_digit) in right.iter().rev().enumerate() {
            let accumulator_index = accumulators
                .len()
                .checked_sub(1 + left_offset + right_offset)?;
            let product = u16::from(*left_digit).checked_mul(u16::from(*right_digit))?;
            accumulators[accumulator_index] =
                accumulators[accumulator_index].checked_add(product)?;
        }
    }

    let mut carry = 0u16;
    for value in accumulators.iter_mut().rev() {
        let total = value.checked_add(carry)?;
        *value = total % 10;
        carry = total / 10;
    }

    while carry > 0 {
        accumulators.insert(0, carry % 10);
        carry /= 10;
    }

    let mut digits = Vec::with_capacity(accumulators.len());
    for value in accumulators {
        digits.push(u8::try_from(value).ok()?);
    }
    trim_decimal_digits(&mut digits);

    Some(digits)
}

pub(super) fn divide_decimal_digits(
    numerator: &[u8],
    denominator: &[u8],
) -> Option<(Vec<u8>, Vec<u8>)> {
    if numerator.is_empty() || denominator.is_empty() {
        return None;
    }
    if is_zero_decimal_digits(denominator) {
        return None;
    }

    let mut quotient = Vec::with_capacity(numerator.len());
    let mut remainder = Vec::new();
    for digit in numerator {
        if !is_zero_decimal_digits(&remainder) || *digit != 0 {
            remainder.push(*digit);
        }
        trim_decimal_digits(&mut remainder);

        let mut quotient_digit = 0u8;
        while compare_decimal_digits(&remainder, denominator) != std::cmp::Ordering::Less {
            remainder = subtract_decimal_digits(&remainder, denominator)?;
            quotient_digit = quotient_digit.checked_add(1)?;
        }
        quotient.push(quotient_digit);
    }

    trim_decimal_digits(&mut quotient);
    trim_decimal_digits(&mut remainder);

    Some((quotient, remainder))
}

pub(super) fn compare_decimal_digits(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
    let left_start = first_nonzero_digit_index(left);
    let right_start = first_nonzero_digit_index(right);
    let left = &left[left_start..];
    let right = &right[right_start..];

    if left.len() != right.len() {
        return left.len().cmp(&right.len());
    }

    left.cmp(right)
}

pub(super) fn first_nonzero_digit_index(digits: &[u8]) -> usize {
    let Some(index) = digits.iter().position(|digit| *digit != 0) else {
        return digits.len().saturating_sub(1);
    };

    index
}

pub(super) fn trim_decimal_digits(digits: &mut Vec<u8>) {
    let index = first_nonzero_digit_index(digits);
    if index > 0 {
        digits.drain(0..index);
    }
    if digits.is_empty() {
        digits.push(0);
    }
}

pub(super) fn is_zero_decimal_digits(digits: &[u8]) -> bool {
    digits.iter().all(|digit| *digit == 0)
}

pub(super) fn decimal_digits_key(digits: &[u8]) -> String {
    let mut key = String::with_capacity(digits.len());
    for digit in digits {
        key.push(char::from(b'0' + *digit));
    }

    key
}

pub(super) fn format_plain_big_decimal(
    negative: bool,
    digits: &[u8],
    scale: usize,
) -> Option<String> {
    if digits.is_empty() {
        return None;
    }

    let mut unsigned = String::new();
    if scale == 0 {
        for digit in digits {
            unsigned.push(char::from(b'0' + *digit));
        }
    } else {
        let padded_len = digits.len().max(scale + 1);
        for _ in 0..padded_len - digits.len() {
            unsigned.push('0');
        }
        for digit in digits {
            unsigned.push(char::from(b'0' + *digit));
        }

        let point_index = unsigned.len().checked_sub(scale)?;
        unsigned.insert(point_index, '.');
    }

    if negative {
        return Some(format!("-{unsigned}"));
    }

    Some(unsigned)
}

pub(super) fn set_big_decimal_scale(
    value: &str,
    scale: i32,
    rounding_mode: &str,
) -> Option<String> {
    let output_scale = usize::try_from(scale).ok()?;
    let parsed = parse_plain_big_decimal(value)?;

    if output_scale >= parsed.scale {
        let mut digits = parsed.digits.clone();
        digits.extend(std::iter::repeat(0).take(output_scale - parsed.scale));
        trim_decimal_digits(&mut digits);
        return format_plain_big_decimal(parsed.negative, &digits, output_scale);
    }

    let rounded_digits = round_scaled_decimal_digits(
        &parsed.digits,
        parsed.scale,
        output_scale,
        rounding_mode,
        parsed.negative,
        false,
    )?;
    let negative = parsed.negative && !is_zero_decimal_digits(&rounded_digits);

    format_plain_big_decimal(negative, &rounded_digits, output_scale)
}

#[cfg(test)]
mod scientific_tests {
    use super::java_scientific_to_plain_decimal;

    // cite: libcore BigDecimal.java#valueOf, layoutChars
    #[test]
    fn libcore_big_decimal_value_of_double_keeps_scale_from_scientific_double_text() {
        for (text, expected) in [
            ("1.0E-5", Some("0.000010")),
            ("-2.5E-4", Some("-0.00025")),
            ("1.0E-6", Some("0.0000010")),
            ("1.0E-7", None),
            ("1.0E7", None),
            ("1.2345E8", None),
            ("0.001", Some("0.001")),
        ] {
            assert_eq!(
                java_scientific_to_plain_decimal(text).as_deref(),
                expected,
                "OpenJDK BigDecimal.valueOf({text}).toString(); None where it needs an exponent"
            );
        }
    }
}
