use super::*;

pub(in crate::dex_run) fn boolean_value_of(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/Boolean;:{}",
        value != 0
    ))))
}

pub(in crate::dex_run) fn boolean_boolean_value(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Reference(reference) = &invocation.arguments[0] else {
        return None;
    };

    if reference.as_ref() == "Ljava/lang/Boolean;:true" {
        return Some(1);
    }

    if reference.as_ref() == "Ljava/lang/Boolean;:false" {
        return Some(0);
    }

    None
}

pub(in crate::dex_run) fn integer_value_of(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/Integer;:{value}"
    ))))
}

pub(in crate::dex_run) fn integer_parse_int(invocation: &DexInvocation, radix: i32) -> Option<i32> {
    if invocation.arguments.is_empty() || !(2..=36).contains(&radix) {
        return None;
    }

    let DexValue::String(value) = &invocation.arguments[0] else {
        return None;
    };

    i32::from_str_radix(value, u32::try_from(radix).ok()?).ok()
}

pub(in crate::dex_run) fn long_parse_long(
    invocation: &DexInvocation,
) -> Option<std::result::Result<i64, String>> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    // cite: libcore Long.java#parseLong
    let value = match &invocation.arguments[0] {
        DexValue::Null => return Some(Err("Cannot parse null string".to_owned())),
        DexValue::String(value) => value,
        _ => return None,
    };
    let for_input_string =
        || -> std::result::Result<i64, String> { Err(format!("For input string: \"{value}\"")) };
    let units: Vec<u16> = value.encode_utf16().collect();
    let Some(&first) = units.first() else {
        return Some(for_input_string());
    };

    let mut negative = false;
    let mut limit = -i64::MAX;
    let mut index = 0;
    if first < u16::from(b'0') {
        if first == u16::from(b'-') {
            negative = true;
            limit = i64::MIN;
        } else if first != u16::from(b'+') {
            return Some(for_input_string());
        }
        if units.len() == 1 {
            return Some(for_input_string());
        }
        index += 1;
    }

    let multmin = limit / 10;
    let mut result = 0i64;
    for &unit in &units[index..] {
        // cite: libcore Character.java#digit
        let digit = match char::from_u32(u32::from(unit)) {
            Some(character) if character.is_ascii() => character.to_digit(10).map_or(-1, i64::from),
            Some(character) if character.is_numeric() => return None,
            _ => -1,
        };
        if digit < 0 || result < multmin {
            return Some(for_input_string());
        }
        result *= 10;
        if result < limit + digit {
            return Some(for_input_string());
        }
        result -= digit;
    }

    Some(Ok(if negative { result } else { -result }))
}

pub(in crate::dex_run) fn double_parse_double(invocation: &DexInvocation) -> Option<i64> {
    let DexValue::String(value) = invocation.arguments.first()? else {
        return None;
    };
    java_parse_double(value).map(|parsed| parsed.to_bits() as i64)
}

// cite: libcore FloatingDecimal.java#readJavaFormatString
// cite: libcore String.java#trim
fn java_parse_double(text: &str) -> Option<f64> {
    let trimmed = text.trim_matches(|character: char| character <= ' ');
    let unsigned = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    if matches!(unsigned, "NaN" | "Infinity") {
        return trimmed.parse::<f64>().ok();
    }
    let number = unsigned
        .strip_suffix(['f', 'F', 'd', 'D'])
        .unwrap_or(unsigned);
    let (mantissa, exponent) = match number.find(['e', 'E']) {
        Some(index) => (&number[..index], Some(&number[index + 1..])),
        None => (number, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty()) || !digits(whole) || !digits(fraction) {
        return None;
    }
    if let Some(exponent) = exponent {
        let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if exponent.is_empty() || !digits(exponent) {
            return None;
        }
    }
    let sign = if trimmed.starts_with('-') { "-" } else { "" };
    format!("{sign}{number}").parse::<f64>().ok()
}

pub(in crate::dex_run) fn long_to_hex_string(invocation: &DexInvocation) -> Option<String> {
    // cite: libcore Long.java#toHexString
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };
    if high_slot_value != value {
        return None;
    }

    Some(format!("{:x}", value as u64))
}

pub(in crate::dex_run) fn integer_to_hex_string(invocation: &DexInvocation) -> Option<String> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(format!("{:x}", value as u32))
}

pub(in crate::dex_run) fn character_digit(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Int(code_unit) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Int(radix) = invocation.arguments[1] else {
        return None;
    };
    let Some(character) = char::from_u32(u32::try_from(code_unit).ok()?) else {
        return Some(-1);
    };
    let radix = u32::try_from(radix).ok()?;
    if !(2..=36).contains(&radix) {
        return Some(-1);
    }

    character
        .to_digit(radix)
        .and_then(|value| i32::try_from(value).ok())
        .or(Some(-1))
}

pub(in crate::dex_run) fn character_value_of(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(code_unit) = invocation.arguments[0] else {
        return None;
    };
    if !(0..=0xffff).contains(&code_unit) {
        return None;
    }

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/Character;:{code_unit}"
    ))))
}

pub(in crate::dex_run) fn character_char_value(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Reference(reference) = &invocation.arguments[0] else {
        return None;
    };

    boxed_character_reference_value(reference)
}

fn boxed_character_reference_value(reference: &str) -> Option<i32> {
    let code_unit = reference
        .strip_prefix("Ljava/lang/Character;:")?
        .parse()
        .ok()?;
    if !(0..=0xffff).contains(&code_unit) {
        return None;
    }

    Some(code_unit)
}

pub(in crate::dex_run) fn character_char_count(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(code_point) = invocation.arguments[0] else {
        return None;
    };

    Some(if code_point >= 0x10000 { 2 } else { 1 })
}

pub(in crate::dex_run) fn character_is_digit(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(code_unit) = invocation.arguments[0] else {
        return None;
    };
    let Ok(code_unit) = u32::try_from(code_unit) else {
        return Some(0);
    };
    let Some(character) = char::from_u32(code_unit) else {
        return Some(0);
    };

    Some(i32::from(character.to_digit(10).is_some()))
}

pub(in crate::dex_run) fn character_is_whitespace(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(code_unit) = invocation.arguments[0] else {
        return None;
    };
    let Ok(code_unit) = u32::try_from(code_unit) else {
        return Some(0);
    };
    let Some(character) = char::from_u32(code_unit) else {
        return Some(0);
    };

    Some(i32::from(character.is_whitespace()))
}

pub(in crate::dex_run) fn character_is_space_char(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(code_unit) = invocation.arguments[0] else {
        return None;
    };
    let Ok(code_unit) = u32::try_from(code_unit) else {
        return Some(0);
    };

    let is_space_char = matches!(
        code_unit,
        0x0020 | 0x00a0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000
    );

    Some(i32::from(is_space_char))
}

pub(in crate::dex_run) fn long_value_of(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };

    if high_slot_value != value {
        return None;
    }

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/Long;:{value}"
    ))))
}

pub(in crate::dex_run) fn long_to_string(
    invocation: &DexInvocation,
    radix_argument_index: Option<usize>,
) -> Option<String> {
    let expected_len = if radix_argument_index.is_some() { 3 } else { 2 };
    if invocation.arguments.len() != expected_len {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };
    if value != high_slot_value {
        return None;
    }

    let radix = match radix_argument_index {
        Some(index) => match invocation.arguments.get(index)? {
            DexValue::Int(radix) if (2..=36).contains(radix) => *radix as u32,
            DexValue::Int(_) => 10,
            _ => return None,
        },
        None => 10,
    };

    Some(format_signed_i64_radix(value, radix))
}

pub(in crate::dex_run) fn format_signed_i64_radix(value: i64, radix: u32) -> String {
    debug_assert!((2..=36).contains(&radix));

    if value == 0 {
        return "0".to_owned();
    }

    let negative = value < 0;
    let mut magnitude = if negative {
        -(value as i128)
    } else {
        value as i128
    };
    let mut digits = Vec::new();

    while magnitude > 0 {
        let digit = (magnitude % i128::from(radix)) as u8;
        let byte = if digit < 10 {
            b'0' + digit
        } else {
            b'a' + digit - 10
        };
        digits.push(byte);
        magnitude /= i128::from(radix);
    }

    if negative {
        digits.push(b'-');
    }
    digits.reverse();

    String::from_utf8(digits).expect("radix digits are valid ASCII")
}

pub(in crate::dex_run) fn float_value_of(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let bits = match invocation.arguments[0] {
        DexValue::Float(value) => value.to_bits(),
        DexValue::Int(value) => value as u32,
        DexValue::Uninitialized
        | DexValue::Null
        | DexValue::Long(_)
        | DexValue::String(_)
        | DexValue::Object(_)
        | DexValue::Reference(_) => return None,
    };

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/Float;:{bits:08x}"
    ))))
}

pub(in crate::dex_run) fn long_get_long(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    let DexValue::String(key) = &invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(default_value) = invocation.arguments[1] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[2] else {
        return None;
    };

    if high_slot_value != default_value {
        return None;
    }

    let value = bootstrap_system_property(key)
        .and_then(|property| java_long_decode(&property))
        .unwrap_or(default_value);

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/Long;:{value}"
    ))))
}

pub(in crate::dex_run) fn boxed_long_reference_value(reference: &str) -> Option<i64> {
    reference.strip_prefix("Ljava/lang/Long;:")?.parse().ok()
}

pub(in crate::dex_run) fn double_value_of(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };
    if high_slot_value != value {
        return None;
    }

    let bits = value as u64;
    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/lang/Double;:{bits:016x}"
    ))))
}

pub(in crate::dex_run) fn boxed_double_reference_value(reference: &str) -> Option<i64> {
    let bits = reference.strip_prefix("Ljava/lang/Double;:")?;
    Some(u64::from_str_radix(bits, 16).ok()? as i64)
}

pub(in crate::dex_run) fn object_equals(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    Some(i32::from(
        invocation.arguments[0] == invocation.arguments[1],
    ))
}

impl DexProgram {
    pub(super) fn comparable_compare_to(&self, invocation: &DexInvocation) -> Option<i32> {
        if invocation.arguments.len() != 2 {
            return None;
        }

        let left = &invocation.arguments[0];
        let right = &invocation.arguments[1];
        let left_descriptor = value_class_descriptor(left)?;
        let right_descriptor = value_class_descriptor(right)?;
        if left_descriptor != right_descriptor {
            return None;
        }

        match left_descriptor {
            "Ljava/lang/String;" => {
                let left = self.string_text(left)?;
                let right = self.string_text(right)?;
                Some(ordering_to_compare_value(left.cmp(&right)))
            }
            "Ljava/lang/Integer;" => {
                let left = self.boxed_integer_compare_value(left)?;
                let right = self.boxed_integer_compare_value(right)?;
                Some(ordering_to_compare_value(left.cmp(&right)))
            }
            "Ljava/lang/Long;" => {
                let left = self.boxed_long_compare_value(left)?;
                let right = self.boxed_long_compare_value(right)?;
                Some(ordering_to_compare_value(left.cmp(&right)))
            }
            "Ljava/lang/Float;" => {
                let left = self.boxed_float_compare_value(left)?;
                let right = self.boxed_float_compare_value(right)?;
                Some(compare_java_float_values(left, right))
            }
            _ => None,
        }
    }

    fn boxed_integer_compare_value(&self, value: &DexValue) -> Option<i32> {
        match value {
            DexValue::Reference(reference) => {
                reference.strip_prefix("Ljava/lang/Integer;:")?.parse().ok()
            }
            value if dex_value_is_object_like(value) => {
                match self.read_stored_instance_field(value, &integer_value_compare_field())? {
                    DexValue::Int(value) => Some(value),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn boxed_long_compare_value(&self, value: &DexValue) -> Option<i64> {
        match value {
            DexValue::Reference(reference) => {
                reference.strip_prefix("Ljava/lang/Long;:")?.parse().ok()
            }
            value if dex_value_is_object_like(value) => {
                match self.read_stored_instance_field(value, &long_value_compare_field())? {
                    DexValue::Long(value) => Some(value),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn boxed_float_compare_value(&self, value: &DexValue) -> Option<f32> {
        match value {
            DexValue::Reference(reference) => boxed_float_compare_reference_value(reference),
            value if dex_value_is_object_like(value) => {
                match self.read_stored_instance_field(value, &float_value_compare_field())? {
                    DexValue::Float(value) => Some(value),
                    DexValue::Int(bits) => Some(f32::from_bits(bits as u32)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

fn ordering_to_compare_value(ordering: std::cmp::Ordering) -> i32 {
    match ordering {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

fn compare_java_float_values(left: f32, right: f32) -> i32 {
    if left < right {
        return -1;
    }
    if left > right {
        return 1;
    }

    ordering_to_compare_value(java_float_to_int_bits(left).cmp(&java_float_to_int_bits(right)))
}

fn boxed_float_compare_reference_value(reference: &str) -> Option<f32> {
    let value = reference.strip_prefix("Ljava/lang/Float;:")?;
    let bits = u32::from_str_radix(value, 16).ok()?;

    Some(f32::from_bits(bits))
}

fn integer_value_compare_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/lang/Integer;".to_owned(),
        type_descriptor: "I".to_owned(),
        name: "value".to_owned(),
    }
}

fn long_value_compare_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/lang/Long;".to_owned(),
        type_descriptor: "J".to_owned(),
        name: "value".to_owned(),
    }
}

fn float_value_compare_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/lang/Float;".to_owned(),
        type_descriptor: "F".to_owned(),
        name: "value".to_owned(),
    }
}

pub(in crate::dex_run) fn objects_require_non_null(invocation: &DexInvocation) -> Option<DexValue> {
    if !matches!(invocation.arguments.len(), 1 | 2) {
        return None;
    }

    let value = invocation.arguments.first()?;

    match value {
        DexValue::Null => None,
        DexValue::Object(_) | DexValue::Reference(_) | DexValue::String(_) => Some(value.clone()),
        DexValue::Uninitialized | DexValue::Int(_) | DexValue::Long(_) | DexValue::Float(_) => None,
    }
}

pub(in crate::dex_run) fn locale_for_language_tag(invocation: &DexInvocation) -> Option<DexValue> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::String(value) = &invocation.arguments[0] else {
        return None;
    };

    Some(DexValue::Reference(std::rc::Rc::from(format!(
        "Ljava/util/Locale;:{value}"
    ))))
}

pub(in crate::dex_run) fn string_value_of_char(invocation: &DexInvocation) -> Option<String> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(char::from_u32(value as u32)?.to_string())
}

pub(in crate::dex_run) fn string_value_of_int(invocation: &DexInvocation) -> Option<String> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(value.to_string())
}

pub(in crate::dex_run) fn string_value_of_long(invocation: &DexInvocation) -> Option<String> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };

    if value != high_slot_value {
        return None;
    }

    Some(value.to_string())
}

pub(in crate::dex_run) fn string_value_of_double(invocation: &DexInvocation) -> Option<String> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(bits) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_bits) = invocation.arguments[1] else {
        return None;
    };

    if bits != high_slot_bits {
        return None;
    }

    java_double_to_string(f64::from_bits(bits as u64))
}

pub(in crate::dex_run) fn math_min_int(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Int(left) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Int(right) = invocation.arguments[1] else {
        return None;
    };

    Some(left.min(right))
}

pub(in crate::dex_run) fn math_min_long(invocation: &DexInvocation) -> Option<i64> {
    let (left, right) = math_two_longs(invocation)?;

    Some(left.min(right))
}

pub(in crate::dex_run) fn math_min_float(invocation: &DexInvocation) -> Option<f32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let left = read_float_argument(&invocation.arguments[0])?;
    let right = read_float_argument(&invocation.arguments[1])?;

    if left.is_nan() || right.is_nan() {
        return Some(f32::NAN);
    }

    if left == right {
        if left == 0.0 && (left.is_sign_negative() || right.is_sign_negative()) {
            return Some(-0.0);
        }

        return Some(left);
    }

    Some(if left < right { left } else { right })
}

pub(in crate::dex_run) fn math_max_int(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Int(left) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Int(right) = invocation.arguments[1] else {
        return None;
    };

    Some(left.max(right))
}

pub(in crate::dex_run) fn math_max_long(invocation: &DexInvocation) -> Option<i64> {
    let (left, right) = math_two_longs(invocation)?;

    Some(left.max(right))
}

pub(in crate::dex_run) fn math_max_float(invocation: &DexInvocation) -> Option<f32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let left = read_float_argument(&invocation.arguments[0])?;
    let right = read_float_argument(&invocation.arguments[1])?;

    if left.is_nan() || right.is_nan() {
        return Some(f32::NAN);
    }

    if left == right {
        if left == 0.0 && (left.is_sign_positive() || right.is_sign_positive()) {
            return Some(0.0);
        }

        return Some(left);
    }

    Some(if left > right { left } else { right })
}

// cite: libcore Math.java#atan2
pub(in crate::dex_run) fn math_atan2_double(invocation: &DexInvocation) -> Option<i64> {
    let (y, x) = double_pair_arguments(invocation)?;

    Some(y.atan2(x).to_bits() as i64)
}

pub(in crate::dex_run) fn math_max_double(invocation: &DexInvocation) -> Option<i64> {
    let (left, right) = double_pair_arguments(invocation)?;

    let value = if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left == right {
        if left == 0.0 && (left.is_sign_positive() || right.is_sign_positive()) {
            0.0
        } else {
            left
        }
    } else if left > right {
        left
    } else {
        right
    };

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_round_float(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let value = match invocation.arguments[0] {
        DexValue::Float(value) => value,
        DexValue::Int(bits) => f32::from_bits(bits as u32),
        _ => return None,
    };

    if value.is_nan() {
        return Some(0);
    }
    if value == f32::INFINITY || value >= i32::MAX as f32 {
        return Some(i32::MAX);
    }
    if value == f32::NEG_INFINITY || value <= i32::MIN as f32 {
        return Some(i32::MIN);
    }

    Some((value + 0.5).floor() as i32)
}

pub(in crate::dex_run) fn math_round_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?;

    if value.is_nan() {
        return Some(0);
    }
    if value == f64::INFINITY || value >= i64::MAX as f64 {
        return Some(i64::MAX);
    }
    if value == f64::NEG_INFINITY || value <= i64::MIN as f64 {
        return Some(i64::MIN);
    }

    Some((value + 0.5).floor() as i64)
}

pub(in crate::dex_run) fn math_signum_float(invocation: &DexInvocation) -> Option<f32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let value = read_float_argument(&invocation.arguments[0])?;
    if value.is_nan() || value == 0.0 {
        return Some(value);
    }

    Some(if value.is_sign_negative() { -1.0 } else { 1.0 })
}

pub(in crate::dex_run) fn single_long_argument(invocation: &DexInvocation) -> Option<i64> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };

    if high_slot_value != value {
        return None;
    }

    Some(value)
}

pub(in crate::dex_run) fn math_two_longs(invocation: &DexInvocation) -> Option<(i64, i64)> {
    if invocation.arguments.len() != 4 {
        return None;
    }

    let DexValue::Long(left) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(left_high_slot) = invocation.arguments[1] else {
        return None;
    };
    let DexValue::Long(right) = invocation.arguments[2] else {
        return None;
    };
    let DexValue::Long(right_high_slot) = invocation.arguments[3] else {
        return None;
    };

    if left != left_high_slot || right != right_high_slot {
        return None;
    }

    Some((left, right))
}

pub(in crate::dex_run) fn integer_number_of_leading_zeros(
    invocation: &DexInvocation,
) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(value.leading_zeros() as i32)
}

pub(in crate::dex_run) fn integer_bit_count(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(value.count_ones() as i32)
}

pub(in crate::dex_run) fn integer_lowest_one_bit(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(value & value.wrapping_neg())
}

pub(in crate::dex_run) fn integer_highest_one_bit(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    if value == 0 {
        return Some(0);
    }

    Some(1_i32.wrapping_shl(31 - value.leading_zeros()))
}

pub(in crate::dex_run) fn integer_hash_code(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(value)
}

pub(in crate::dex_run) fn integer_signum(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(match value.cmp(&0) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })
}

pub(in crate::dex_run) fn long_number_of_trailing_zeros(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };

    if value != high_slot_value {
        return None;
    }

    Some(value.trailing_zeros() as i32)
}

pub(in crate::dex_run) fn long_number_of_leading_zeros(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };

    if value != high_slot_value {
        return None;
    }

    Some(value.leading_zeros() as i32)
}

pub(in crate::dex_run) fn long_hash_code(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };

    if value != high_slot_value {
        return None;
    }

    let unsigned = value as u64;
    Some((unsigned ^ (unsigned >> 32)) as u32 as i32)
}

pub(in crate::dex_run) fn long_signum(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };

    if value != high_slot_value {
        return None;
    }

    Some(match value.cmp(&0) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })
}

pub(in crate::dex_run) fn long_compare_unsigned(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 4 {
        return None;
    }

    let DexValue::Long(left) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(left_high_slot) = invocation.arguments[1] else {
        return None;
    };
    let DexValue::Long(right) = invocation.arguments[2] else {
        return None;
    };
    let DexValue::Long(right_high_slot) = invocation.arguments[3] else {
        return None;
    };

    if left != left_high_slot || right != right_high_slot {
        return None;
    }

    Some(match (left as u64).cmp(&(right as u64)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    })
}

pub(in crate::dex_run) fn long_rotate_left(invocation: &DexInvocation) -> Option<i64> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };
    let DexValue::Int(distance) = invocation.arguments[2] else {
        return None;
    };

    if value != high_slot_value {
        return None;
    }

    Some(value.rotate_left(distance as u32))
}

pub(in crate::dex_run) fn long_rotate_right(invocation: &DexInvocation) -> Option<i64> {
    if invocation.arguments.len() != 3 {
        return None;
    }

    let DexValue::Long(value) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_value) = invocation.arguments[1] else {
        return None;
    };
    let DexValue::Int(distance) = invocation.arguments[2] else {
        return None;
    };

    if value != high_slot_value {
        return None;
    }

    Some(value.rotate_right(distance as u32))
}

pub(in crate::dex_run) fn float_int_bits_to_float(invocation: &DexInvocation) -> Option<f32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(bits) = invocation.arguments[0] else {
        return None;
    };

    Some(f32::from_bits(bits as u32))
}

pub(in crate::dex_run) fn float_to_raw_int_bits(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    Some(read_float_argument(&invocation.arguments[0])?.to_bits() as i32)
}

pub(in crate::dex_run) fn float_to_int_bits(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    Some(java_float_to_int_bits(read_float_argument(
        &invocation.arguments[0],
    )?))
}

pub(in crate::dex_run) fn float_compare(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let left = read_float_argument(&invocation.arguments[0])?;
    let right = read_float_argument(&invocation.arguments[1])?;

    if left < right {
        return Some(-1);
    }
    if left > right {
        return Some(1);
    }

    let left_bits = java_float_to_int_bits(left);
    let right_bits = java_float_to_int_bits(right);
    Some(left_bits.cmp(&right_bits) as i32)
}

pub(in crate::dex_run) fn float_is_nan(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    Some(i32::from(
        read_float_argument(&invocation.arguments[0])?.is_nan(),
    ))
}

pub(in crate::dex_run) fn float_is_infinite(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    Some(i32::from(
        read_float_argument(&invocation.arguments[0])?.is_infinite(),
    ))
}

pub(in crate::dex_run) fn double_is_nan(invocation: &DexInvocation) -> Option<i32> {
    Some(i32::from(single_double_argument(invocation)?.is_nan()))
}

pub(in crate::dex_run) fn double_to_raw_long_bits(invocation: &DexInvocation) -> Option<i64> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    let DexValue::Long(bits) = invocation.arguments[0] else {
        return None;
    };
    let DexValue::Long(high_slot_bits) = invocation.arguments[1] else {
        return None;
    };

    if bits != high_slot_bits {
        return None;
    }

    Some(bits)
}

pub(in crate::dex_run) fn math_abs_float(invocation: &DexInvocation) -> Option<f32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    Some(read_float_argument(&invocation.arguments[0])?.abs())
}

pub(in crate::dex_run) fn math_abs_int(invocation: &DexInvocation) -> Option<i32> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    let DexValue::Int(value) = invocation.arguments[0] else {
        return None;
    };

    Some(value.wrapping_abs())
}

pub(in crate::dex_run) fn read_float_argument(value: &DexValue) -> Option<f32> {
    match value {
        DexValue::Float(value) => Some(*value),
        DexValue::Int(bits) => Some(f32::from_bits(*bits as u32)),
        DexValue::Uninitialized
        | DexValue::Null
        | DexValue::Long(_)
        | DexValue::String(_)
        | DexValue::Object(_)
        | DexValue::Reference(_) => None,
    }
}

pub(in crate::dex_run) fn java_float_to_string(value: f32) -> Option<String> {
    // cite: libcore Float.java#toString
    if value.is_nan() {
        return Some("NaN".to_owned());
    }
    if value == f32::INFINITY {
        return Some("Infinity".to_owned());
    }
    if value == f32::NEG_INFINITY {
        return Some("-Infinity".to_owned());
    }

    let mut value = value.to_string();
    if !value.contains('.') && !value.contains('e') && !value.contains('E') {
        value.push_str(".0");
    }

    Some(value)
}

pub(in crate::dex_run) fn java_float_to_int_bits(value: f32) -> i32 {
    if value.is_nan() {
        return 0x7fc0_0000;
    }

    value.to_bits() as i32
}

pub(in crate::dex_run) fn thread_get_id(invocation: &DexInvocation) -> Option<()> {
    if invocation.arguments.len() != 1 {
        return None;
    }

    match &invocation.arguments[0] {
        DexValue::Reference(reference) if reference.as_ref() == "Ljava/lang/Thread;:main" => {
            Some(())
        }
        DexValue::Object(_) | DexValue::Reference(_) => Some(()),
        DexValue::Uninitialized
        | DexValue::Null
        | DexValue::Int(_)
        | DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::String(_) => None,
    }
}

pub(in crate::dex_run) fn math_abs_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.abs();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_ceil_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.ceil();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_floor_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.floor();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_rint_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.round_ties_even();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_log_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.ln();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_exp_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.exp();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_log10_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.log10();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_ulp_double(invocation: &DexInvocation) -> Option<i64> {
    let value = java_math_ulp(single_double_argument(invocation)?);

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_sqrt_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.sqrt();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn java_math_ulp(value: f64) -> f64 {
    if value.is_nan() {
        return value.abs();
    }

    if value.is_infinite() {
        return f64::INFINITY;
    }

    let value = value.abs();
    if value == 0.0 {
        return f64::from_bits(1);
    }

    let exponent_bits = ((value.to_bits() >> 52) & 0x7ff) as i32;
    if exponent_bits == 0 {
        return f64::from_bits(1);
    }

    let exponent = exponent_bits - 1023;
    let ulp_exponent = exponent - 52;
    if ulp_exponent >= -1022 {
        return f64::from_bits(((ulp_exponent + 1023) as u64) << 52);
    }

    f64::from_bits(1_u64 << ((ulp_exponent + 1074) as u32))
}

pub(in crate::dex_run) fn math_to_radians_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.to_radians();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_to_degrees_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.to_degrees();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_sin_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.sin();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_asin_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.asin();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_cos_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.cos();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_acos_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.acos();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_tan_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.tan();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_atan_double(invocation: &DexInvocation) -> Option<i64> {
    let value = single_double_argument(invocation)?.atan();

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_pow_double(invocation: &DexInvocation) -> Option<i64> {
    let (base, exponent) = double_pair_arguments(invocation)?;
    let value = base.powf(exponent);

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn math_hypot_double(invocation: &DexInvocation) -> Option<i64> {
    let (left, right) = double_pair_arguments(invocation)?;
    let value = left.hypot(right);

    Some(value.to_bits() as i64)
}

pub(in crate::dex_run) fn single_double_argument(invocation: &DexInvocation) -> Option<f64> {
    if invocation.arguments.len() != 2 {
        return None;
    }

    read_wide_double(&invocation.arguments[0], &invocation.arguments[1])
}

pub(in crate::dex_run) fn double_pair_arguments(invocation: &DexInvocation) -> Option<(f64, f64)> {
    if invocation.arguments.len() != 4 {
        return None;
    }

    let first = read_wide_double(&invocation.arguments[0], &invocation.arguments[1])?;
    let second = read_wide_double(&invocation.arguments[2], &invocation.arguments[3])?;

    Some((first, second))
}

pub(in crate::dex_run) fn read_wide_double(low: &DexValue, high: &DexValue) -> Option<f64> {
    let DexValue::Long(bits) = low else {
        return None;
    };
    let DexValue::Long(high_slot_bits) = high else {
        return None;
    };

    if bits != high_slot_bits {
        return None;
    }

    Some(f64::from_bits(*bits as u64))
}
