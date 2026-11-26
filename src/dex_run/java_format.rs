use super::*;

#[derive(Default, Clone, Copy)]
struct FormatFlags {
    left_justify: bool,
    plus: bool,
    leading_space: bool,
    zero_pad: bool,
    alternate: bool,
    grouping: bool,
    parentheses: bool,
    previous: bool,
}

struct FormatSpecifier {
    index: Option<usize>,
    flags: FormatFlags,
    width: Option<usize>,
    precision: Option<usize>,
    upper: bool,
    date_time: bool,
    conversion: char,
}

impl FormatSpecifier {
    // cite: libcore Formatter.java#FormatSpecifierParser, FLAGS
    fn parse(format: &[char], cursor: &mut usize) -> Option<Self> {
        let mut specifier = Self {
            index: None,
            flags: FormatFlags::default(),
            width: None,
            precision: None,
            upper: false,
            date_time: false,
            conversion: ' ',
        };

        let digit_run = |cursor: &mut usize| {
            let start = *cursor;
            while format.get(*cursor).is_some_and(char::is_ascii_digit) {
                *cursor += 1;
            }
            (start, *cursor)
        };

        let (start, end) = digit_run(cursor);
        if start != end {
            if format.get(end) == Some(&'$') {
                let digits: String = format[start..end].iter().collect();
                specifier.index = Some(digits.parse().ok()?);
                *cursor = end + 1;
            } else if format[start] == '0' {
                *cursor = start;
            } else {
                let digits: String = format[start..end].iter().collect();
                specifier.width = Some(digits.parse().ok()?);
                *cursor = end;
            }
        }

        if specifier.width.is_none() {
            while let Some(character) = format.get(*cursor) {
                let recognized = match character {
                    '-' => {
                        specifier.flags.left_justify = true;
                        true
                    }
                    '+' => {
                        specifier.flags.plus = true;
                        true
                    }
                    ' ' => {
                        specifier.flags.leading_space = true;
                        true
                    }
                    '0' => {
                        specifier.flags.zero_pad = true;
                        true
                    }
                    '#' => {
                        specifier.flags.alternate = true;
                        true
                    }
                    ',' => {
                        specifier.flags.grouping = true;
                        true
                    }
                    '(' => {
                        specifier.flags.parentheses = true;
                        true
                    }
                    '<' => {
                        specifier.flags.previous = true;
                        true
                    }
                    _ => false,
                };
                if !recognized {
                    break;
                }
                *cursor += 1;
            }
        }

        if specifier.width.is_none() {
            let (start, end) = digit_run(cursor);
            if start != end {
                let digits: String = format[start..end].iter().collect();
                specifier.width = Some(digits.parse().ok()?);
                *cursor = end;
            }
        }

        if format.get(*cursor) == Some(&'.') {
            *cursor += 1;
            let (start, end) = digit_run(cursor);
            if start == end {
                return None;
            }
            let digits: String = format[start..end].iter().collect();
            specifier.precision = Some(digits.parse().ok()?);
            *cursor = end;
        }

        if matches!(format.get(*cursor), Some('t' | 'T')) {
            specifier.date_time = true;
            specifier.upper = format[*cursor] == 'T';
            *cursor += 1;
            let conversion = *format.get(*cursor)?;
            *cursor += 1;
            specifier.conversion = conversion;
            return Some(specifier);
        }

        let conversion = *format.get(*cursor)?;
        *cursor += 1;
        if conversion.is_ascii_uppercase() {
            specifier.upper = true;
            specifier.conversion = conversion.to_ascii_lowercase();
        } else {
            specifier.conversion = conversion;
        }

        Some(specifier)
    }
}

impl DexProgram {
    // cite: libcore Formatter.java#format, parse
    pub(super) fn format_java_string(
        &self,
        format: &str,
        arguments: Option<&[DexValue]>,
    ) -> Option<String> {
        let characters: Vec<char> = format.chars().collect();
        let mut result = String::new();
        let mut cursor = 0usize;
        let mut last_argument = None;
        let mut last_ordinary_argument = None;

        while cursor < characters.len() {
            if characters[cursor] != '%' {
                result.push(characters[cursor]);
                cursor += 1;
                continue;
            }

            cursor += 1;
            let specifier = FormatSpecifier::parse(&characters, &mut cursor)?;

            if specifier.conversion == '%' {
                result.push('%');
                continue;
            }
            if specifier.conversion == 'n' {
                result.push('\n');
                continue;
            }

            let (argument_index, next_ordinary) = match specifier.index {
                Some(index) => {
                    let index = index.checked_sub(1)?;
                    (index, last_ordinary_argument)
                }
                None if specifier.flags.previous => (last_argument?, last_ordinary_argument),
                None => {
                    let index = last_ordinary_argument.map_or(0, |value| value + 1);
                    (index, Some(index))
                }
            };
            last_argument = Some(argument_index);
            last_ordinary_argument = next_ordinary;

            let argument = match arguments {
                None => None,
                Some(values) => Some(values.get(argument_index)?.clone()),
            };

            let formatted = self.format_specifier(&specifier, argument)?;
            result.push_str(&formatted);
        }

        Some(result)
    }

    fn format_specifier(
        &self,
        specifier: &FormatSpecifier,
        argument: Option<DexValue>,
    ) -> Option<String> {
        if specifier.date_time {
            return None;
        }

        match specifier.conversion {
            's' => {
                if specifier.flags.alternate {
                    return None;
                }
                let text = match argument {
                    None => "null".to_owned(),
                    Some(value) => self.format_string_conversion_value(&value)?,
                };
                Some(apply_string_format(text, specifier))
            }
            'b' => {
                let text = match argument {
                    Some(DexValue::Reference(reference))
                        if reference.as_ref() == "Ljava/lang/Boolean;:true" =>
                    {
                        "true".to_owned()
                    }
                    Some(DexValue::Reference(reference))
                        if reference.as_ref() == "Ljava/lang/Boolean;:false" =>
                    {
                        "false".to_owned()
                    }
                    Some(DexValue::Null) | None => "false".to_owned(),
                    Some(_) => "true".to_owned(),
                };
                Some(apply_string_format(text, specifier))
            }
            'h' => {
                let text = match argument {
                    Some(DexValue::Null) | None => "null".to_owned(),
                    Some(value) => format_hash_code(&value)?,
                };
                Some(apply_string_format(text, specifier))
            }
            'c' => {
                let text = match argument {
                    Some(DexValue::Null) | None => "null".to_owned(),
                    Some(value) => {
                        let code = format_character_code(&value)?;
                        char::from_u32(code)?.to_string()
                    }
                };
                Some(apply_string_format(text, specifier))
            }
            'd' | 'o' | 'x' => {
                let text = match argument {
                    Some(DexValue::Null) | None => {
                        apply_string_format("null".to_owned(), specifier)
                    }
                    Some(value) => {
                        let integer = format_integer_value(&value)?;
                        format_integer(specifier, integer)?
                    }
                };
                Some(text)
            }
            'f' | 'e' | 'g' | 'a' => {
                let text = match argument {
                    Some(DexValue::Null) | None => format_justified("null".to_owned(), specifier),
                    Some(value) => {
                        if specifier.conversion == 'a' {
                            return None;
                        }
                        let number = format_float_value(&value)?;
                        format_float(specifier, number)?
                    }
                };
                Some(text)
            }
            _ => None,
        }
    }

    fn format_string_conversion_value(&self, value: &DexValue) -> Option<String> {
        match value {
            DexValue::String(text) => Some(text.clone().to_string()),
            DexValue::Int(value) => Some(value.to_string()),
            DexValue::Long(value) => Some(value.to_string()),
            DexValue::Float(value) => java_float_to_string(*value),
            DexValue::Null => Some("null".to_owned()),
            DexValue::Reference(_) | DexValue::Object(_) => self.object_value_to_string(value),
            DexValue::Uninitialized => None,
        }
    }
}

fn apply_string_format(mut text: String, specifier: &FormatSpecifier) -> String {
    if let Some(precision) = specifier.precision {
        text = text.chars().take(precision).collect();
    }
    if specifier.upper {
        text = text.to_uppercase();
    }
    format_justified(text, specifier)
}

fn format_justified(text: String, specifier: &FormatSpecifier) -> String {
    let width = specifier.width.unwrap_or(0);
    let length = text.chars().count();
    if length >= width {
        return text;
    }
    let padding = width - length;
    if specifier.flags.left_justify {
        let mut text = text;
        text.extend(std::iter::repeat(' ').take(padding));
        text
    } else {
        let mut justified = String::with_capacity(width);
        justified.extend(std::iter::repeat(' ').take(padding));
        justified.push_str(&text);
        justified
    }
}

fn format_hash_code(value: &DexValue) -> Option<String> {
    let hash = match value {
        DexValue::String(text) => java_string_hash_code(text),
        DexValue::Int(value) => *value,
        DexValue::Long(value) => (*value ^ ((*value as u64 >> 32) as i64)) as i32,
        DexValue::Float(value) => value.to_bits() as i32,
        DexValue::Reference(reference) => {
            if reference.as_ref() == "Ljava/lang/Boolean;:true" {
                1231
            } else if reference.as_ref() == "Ljava/lang/Boolean;:false" {
                1237
            } else if let Some(value) = boxed_integer_reference_value(reference) {
                if let Some(bits) = reference.strip_prefix("Ljava/lang/Long;:") {
                    let _ = bits;
                    (value ^ ((value as u64 >> 32) as i64)) as i32
                } else {
                    value as i32
                }
            } else if let Some(value) = boxed_float_reference_value(reference) {
                value.to_bits() as i32
            } else if let Some(value) = boxed_double_reference_value(reference) {
                let value = value as u64;
                (value ^ (value >> 32)) as i32
            } else {
                stable_hash_i32(&map_identity_key(value))
            }
        }
        DexValue::Object(_) => stable_hash_i32(&map_identity_key(value)),
        DexValue::Null | DexValue::Uninitialized => return None,
    };

    Some(format!("{:x}", hash))
}

fn java_string_hash_code(text: &str) -> i32 {
    let mut hash = 0i32;
    for code_unit in text.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(i32::from(code_unit));
    }
    hash
}

fn format_character_code(value: &DexValue) -> Option<u32> {
    let code = match value {
        DexValue::Int(value) => *value,
        DexValue::Reference(reference) => {
            if let Some(value) = boxed_integer_reference_value(reference) {
                value as i32
            } else {
                return None;
            }
        }
        DexValue::Long(_)
        | DexValue::Float(_)
        | DexValue::String(_)
        | DexValue::Object(_)
        | DexValue::Null
        | DexValue::Uninitialized => return None,
    };

    u32::try_from(code).ok()
}

#[derive(Clone, Copy)]
struct IntegerValue {
    value: i64,
    bits: u32,
}

fn format_integer_value(value: &DexValue) -> Option<IntegerValue> {
    match value {
        DexValue::Int(value) => Some(IntegerValue {
            value: i64::from(*value),
            bits: 32,
        }),
        DexValue::Long(value) => Some(IntegerValue {
            value: *value,
            bits: 64,
        }),
        DexValue::Reference(reference) => {
            let value = boxed_integer_reference_value(reference)?;
            let bits = if reference.starts_with("Ljava/lang/Long;:") {
                64
            } else {
                32
            };
            Some(IntegerValue { value, bits })
        }
        DexValue::Float(_)
        | DexValue::String(_)
        | DexValue::Object(_)
        | DexValue::Null
        | DexValue::Uninitialized => None,
    }
}

fn format_integer(specifier: &FormatSpecifier, value: IntegerValue) -> Option<String> {
    let Some(radix) = integer_radix(specifier.conversion) else {
        return None;
    };
    if specifier.precision.is_some() {
        return None;
    }

    if radix != 10 {
        if specifier.flags.plus || specifier.flags.leading_space || specifier.flags.parentheses {
            return None;
        }
        return Some(format_unsigned_radix(specifier, value, radix));
    }

    if specifier.flags.alternate {
        return None;
    }

    let negative = value.value < 0;
    let magnitude = (value.value as i128).unsigned_abs();
    let mut digits = magnitude.to_string();
    if specifier.flags.grouping {
        digits = group_decimal_digits(&digits);
    }

    let mut content = String::new();
    if negative {
        if specifier.flags.parentheses {
            content.push('(');
        } else {
            content.push('-');
        }
    } else if specifier.flags.plus {
        content.push('+');
    } else if specifier.flags.leading_space {
        content.push(' ');
    }
    content.push_str(&digits);
    if negative && specifier.flags.parentheses {
        content.push(')');
    }

    Some(justify_numeric(content, specifier))
}

fn integer_radix(conversion: char) -> Option<u32> {
    match conversion {
        'd' => Some(10),
        'o' => Some(8),
        'x' => Some(16),
        _ => None,
    }
}

fn format_unsigned_radix(specifier: &FormatSpecifier, value: IntegerValue, radix: u32) -> String {
    let unsigned = if value.bits == 32 {
        u64::from(value.value as u32)
    } else {
        value.value as u64
    };
    let digits = match radix {
        8 => format!("{unsigned:o}"),
        16 => {
            if specifier.upper {
                format!("{unsigned:X}")
            } else {
                format!("{unsigned:x}")
            }
        }
        _ => unreachable!(),
    };

    let prefix = if specifier.flags.alternate {
        match radix {
            8 => "0",
            16 => {
                if specifier.upper {
                    "0X"
                } else {
                    "0x"
                }
            }
            _ => "",
        }
    } else {
        ""
    };

    let mut content = String::new();
    content.push_str(prefix);
    if specifier.flags.zero_pad && !specifier.flags.left_justify {
        if let Some(width) = specifier.width {
            let length = prefix.chars().count() + digits.chars().count();
            if length < width {
                content.extend(std::iter::repeat('0').take(width - length));
            }
        }
    }
    content.push_str(&digits);

    justify_numeric(content, specifier)
}

fn group_decimal_digits(digits: &str) -> String {
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(character);
    }
    grouped
}

fn justify_numeric(content: String, specifier: &FormatSpecifier) -> String {
    let width = specifier.width.unwrap_or(0);
    let length = content.chars().count();
    if length >= width {
        return content;
    }
    let padding = width - length;
    if specifier.flags.left_justify {
        let mut content = content;
        content.extend(std::iter::repeat(' ').take(padding));
        content
    } else if specifier.flags.zero_pad {
        let mut padded = String::with_capacity(width);
        let (sign, rest) = split_numeric_sign(&content);
        padded.push_str(sign);
        padded.extend(std::iter::repeat('0').take(padding));
        padded.push_str(rest);
        padded
    } else {
        let mut padded = String::with_capacity(width);
        padded.extend(std::iter::repeat(' ').take(padding));
        padded.push_str(&content);
        padded
    }
}

fn split_numeric_sign(content: &str) -> (&str, &str) {
    if let Some(rest) = content.strip_prefix('(') {
        return ("(", rest);
    }
    if let Some(rest) = content.strip_prefix('-') {
        return ("-", rest);
    }
    if let Some(rest) = content.strip_prefix('+') {
        return ("+", rest);
    }
    if let Some(rest) = content.strip_prefix(' ') {
        return (" ", rest);
    }
    ("", content)
}

fn format_float_value(value: &DexValue) -> Option<f64> {
    match value {
        DexValue::Float(value) => Some(f64::from(*value)),
        DexValue::Reference(reference) => {
            if let Some(value) = boxed_float_reference_value(reference) {
                Some(f64::from(value))
            } else {
                let bits = boxed_double_reference_value(reference)?;
                Some(f64::from_bits(bits as u64))
            }
        }
        DexValue::Long(_)
        | DexValue::Int(_)
        | DexValue::String(_)
        | DexValue::Object(_)
        | DexValue::Null
        | DexValue::Uninitialized => None,
    }
}

fn format_float(specifier: &FormatSpecifier, value: f64) -> Option<String> {
    if value.is_nan() {
        let text = if specifier.upper { "NAN" } else { "NaN" };
        return Some(format_justified(text.to_owned(), specifier));
    }
    if value.is_infinite() {
        let text = if specifier.upper {
            "INFINITY"
        } else {
            "Infinity"
        };
        let text = if value.is_sign_negative() {
            format!("-{text}")
        } else if specifier.flags.plus {
            format!("+{text}")
        } else if specifier.flags.leading_space {
            format!(" {text}")
        } else {
            text.to_owned()
        };
        return Some(format_justified(text, specifier));
    }

    let negative = value.is_sign_negative();
    if negative && specifier.flags.parentheses {
        let mut content = format_finite_float(specifier, value.abs())?;
        content.insert(0, '(');
        content.push(')');
        return Some(justify_float(content, specifier));
    }

    let mut content = String::new();
    if negative {
        content.push('-');
    } else if specifier.flags.plus {
        content.push('+');
    } else if specifier.flags.leading_space {
        content.push(' ');
    }
    content.push_str(&format_finite_float(specifier, value.abs())?);

    if specifier.upper {
        content = content.to_uppercase();
    }

    Some(justify_float(content, specifier))
}

fn justify_float(content: String, specifier: &FormatSpecifier) -> String {
    let width = specifier.width.unwrap_or(0);
    let length = content.chars().count();
    if length >= width {
        return content;
    }
    let padding = width - length;
    if specifier.flags.left_justify {
        let mut content = content;
        content.extend(std::iter::repeat(' ').take(padding));
        content
    } else if specifier.flags.zero_pad {
        let (sign, rest) = split_numeric_sign(&content);
        let mut padded = String::with_capacity(width);
        padded.push_str(sign);
        padded.extend(std::iter::repeat('0').take(padding));
        padded.push_str(rest);
        padded
    } else {
        let mut padded = String::with_capacity(width);
        padded.extend(std::iter::repeat(' ').take(padding));
        padded.push_str(&content);
        padded
    }
}

fn format_finite_float(specifier: &FormatSpecifier, value: f64) -> Option<String> {
    let precision = specifier.precision;
    let text = match specifier.conversion {
        'f' => {
            let digits = precision.unwrap_or(6);
            let mut text = format!("{value:.digits$}");
            if specifier.flags.alternate && digits == 0 {
                text.push('.');
            }
            text
        }
        'e' => {
            let digits = precision.unwrap_or(6);
            let text = format!("{value:.digits$e}");
            let (mantissa, exponent) = text.split_once('e')?;
            let exponent: i32 = exponent.parse().ok()?;
            let mut text = String::new();
            if specifier.flags.alternate && digits == 0 {
                text.push_str(mantissa);
                text.push('.');
            } else {
                text.push_str(mantissa);
            }
            text.push('e');
            if exponent < 0 {
                text.push('-');
            } else {
                text.push('+');
            }
            let exponent = exponent.unsigned_abs().to_string();
            text.extend(std::iter::repeat('0').take(2usize.saturating_sub(exponent.len())));
            text.push_str(&exponent);
            text
        }
        'g' => format_general_float(specifier, value)?,
        _ => return None,
    };

    if !specifier.flags.grouping {
        return Some(text);
    }

    let (integer_part, fraction) = text.split_once('.').unwrap_or((text.as_str(), ""));
    let exponent_index = integer_part
        .find('e')
        .or_else(|| integer_part.find('E'))
        .unwrap_or(integer_part.len());
    let (integer, exponent) = integer_part.split_at(exponent_index);
    let grouped = group_decimal_digits(integer);
    if fraction.is_empty() && exponent.is_empty() {
        return Some(grouped);
    }
    if exponent.is_empty() {
        return Some(format!("{grouped}.{fraction}"));
    }
    Some(format!("{grouped}{exponent}.{fraction}"))
}

fn format_general_float(specifier: &FormatSpecifier, value: f64) -> Option<String> {
    let digits = match specifier.precision {
        Some(0) => 1,
        Some(value) => value,
        None => 6,
    };

    if value == 0.0 {
        let mut text = "0".to_owned();
        if specifier.flags.alternate || digits > 1 {
            text.push('.');
            if digits > 1 {
                text.extend(std::iter::repeat('0').take(digits - 1));
            }
        }
        return Some(text);
    }

    let scientific = format!("{value:.*e}", digits.saturating_sub(1));
    let (mantissa, exponent) = scientific.split_once('e')?;
    let exponent: i32 = exponent.parse().ok()?;

    let text = if (-4..digits as i32).contains(&exponent) {
        let fraction_digits = (digits as i32 - exponent - 1).max(0) as usize;
        format!("{value:.fraction_digits$}")
    } else {
        let mut text = mantissa.to_owned();
        text.push('e');
        if exponent < 0 {
            text.push('-');
        } else {
            text.push('+');
        }
        let magnitude = exponent.unsigned_abs().to_string();
        text.extend(std::iter::repeat('0').take(2usize.saturating_sub(magnitude.len())));
        text.push_str(&magnitude);
        text
    };

    Some(text)
}
