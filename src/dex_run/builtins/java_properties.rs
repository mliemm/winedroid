use super::super::*;

const LINE_FEED: u16 = 0x0a;
const CARRIAGE_RETURN: u16 = 0x0d;
const BACKSLASH: u16 = b'\\' as u16;

enum PropertiesLoadError {
    MalformedUnicodeEscape,
    IndexOutOfBounds { length: usize, index: usize },
}

struct PropertiesLineReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    line_buf: Vec<u16>,
}

fn is_whitespace(c: u16) -> bool {
    // cite: libcore Character.java#isWhitespace
    (0x1c..=0x20).contains(&c) || (0x09..=0x0d).contains(&c)
}

impl<'a> PropertiesLineReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            line_buf: vec![0; 1024],
        }
    }

    fn read_line(&mut self) -> Option<usize> {
        // cite: libcore Properties.java#LineReader, readLine
        let mut len = 0usize;
        let mut skip_white_space = true;
        let mut is_comment_line = false;
        let mut is_new_line = true;
        let mut appended_line_begin = false;
        let mut preceding_backslash = false;
        let mut skip_lf = false;

        loop {
            let Some(&byte) = self.bytes.get(self.offset) else {
                if len == 0 || is_comment_line {
                    return None;
                }
                return Some(if preceding_backslash { len - 1 } else { len });
            };
            self.offset += 1;
            let c = u16::from(byte);

            if skip_lf {
                skip_lf = false;
                if c == LINE_FEED {
                    continue;
                }
            }
            if skip_white_space {
                if is_whitespace(c) {
                    continue;
                }
                if !appended_line_begin && (c == CARRIAGE_RETURN || c == LINE_FEED) {
                    continue;
                }
                skip_white_space = false;
                appended_line_begin = false;
            }
            if is_new_line {
                is_new_line = false;
                if c == u16::from(b'#') || c == u16::from(b'!') {
                    is_comment_line = true;
                    continue;
                }
            }

            if c != LINE_FEED && c != CARRIAGE_RETURN {
                self.line_buf[len] = c;
                len += 1;
                if len == self.line_buf.len() {
                    self.line_buf.resize(len * 2, 0);
                }
                preceding_backslash = c == BACKSLASH && !preceding_backslash;
                continue;
            }

            if is_comment_line || len == 0 {
                is_comment_line = false;
                is_new_line = true;
                skip_white_space = true;
                len = 0;
                continue;
            }
            if self.offset >= self.bytes.len() {
                return Some(if preceding_backslash { len - 1 } else { len });
            }
            if !preceding_backslash {
                return Some(len);
            }
            len -= 1;
            skip_white_space = true;
            appended_line_begin = true;
            preceding_backslash = false;
            if c == CARRIAGE_RETURN {
                skip_lf = true;
            }
        }
    }
}

fn load_convert(
    buffer: &[u16],
    offset: usize,
    length: usize,
) -> std::result::Result<String, PropertiesLoadError> {
    // cite: libcore Properties.java#loadConvert
    let at = |index: usize| {
        buffer
            .get(index)
            .copied()
            .ok_or(PropertiesLoadError::IndexOutOfBounds {
                length: buffer.len(),
                index,
            })
    };
    let end = offset + length;
    let mut index = offset;
    let mut out = Vec::with_capacity(length);

    while index < end {
        let mut c = at(index)?;
        index += 1;
        if c != BACKSLASH {
            out.push(c);
            continue;
        }
        c = at(index)?;
        index += 1;
        if c == u16::from(b'u') {
            let mut value = 0u16;
            for _ in 0..4 {
                let digit = char::from_u32(u32::from(at(index)?))
                    .and_then(|digit| digit.to_digit(16))
                    .ok_or(PropertiesLoadError::MalformedUnicodeEscape)?;
                index += 1;
                value = (value << 4) + digit as u16;
            }
            out.push(value);
            continue;
        }
        out.push(match u8::try_from(c).ok() {
            Some(b't') => 0x09,
            Some(b'r') => CARRIAGE_RETURN,
            Some(b'n') => LINE_FEED,
            Some(b'f') => 0x0c,
            _ => c,
        });
    }

    Ok(String::from_utf16_lossy(&out))
}

fn parse_properties_latin1(bytes: &[u8]) -> (Vec<(String, String)>, Option<PropertiesLoadError>) {
    // cite: libcore Properties.java#load0
    let mut reader = PropertiesLineReader::new(bytes);
    let mut entries = Vec::new();

    while let Some(limit) = reader.read_line() {
        let mut key_len = 0usize;
        let mut value_start = limit;
        let mut has_sep = false;
        let mut preceding_backslash = false;

        while key_len < limit {
            let c = reader.line_buf[key_len];
            if (c == u16::from(b'=') || c == u16::from(b':')) && !preceding_backslash {
                value_start = key_len + 1;
                has_sep = true;
                break;
            }
            if is_whitespace(c) && !preceding_backslash {
                value_start = key_len + 1;
                break;
            }
            preceding_backslash = c == BACKSLASH && !preceding_backslash;
            key_len += 1;
        }
        while value_start < limit {
            let c = reader.line_buf[value_start];
            if !is_whitespace(c) {
                if !has_sep && (c == u16::from(b'=') || c == u16::from(b':')) {
                    has_sep = true;
                } else {
                    break;
                }
            }
            value_start += 1;
        }

        let converted = load_convert(&reader.line_buf, 0, key_len).and_then(|key| {
            load_convert(&reader.line_buf, value_start, limit - value_start)
                .map(|value| (key, value))
        });
        match converted {
            Ok(entry) => entries.push(entry),
            Err(error) => return (entries, Some(error)),
        }
    }

    (entries, None)
}

impl DexProgram {
    pub(super) fn properties_load_from_stream(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        // cite: libcore Properties.java#load
        let [receiver, stream] = invocation.arguments.as_slice() else {
            return None;
        };
        if !dex_value_is_object_like(receiver) {
            return None;
        }
        if *stream == DexValue::Null {
            return Some(self.null_pointer_exception_result());
        }

        let bytes = self.drain_input_stream_bytes(stream)?;
        let (entries, error) = parse_properties_latin1(&bytes);
        for (key, value) in entries {
            let put = DexInvocation {
                kind: DexInvokeKind::Virtual,
                range: false,
                method_index: 0,
                argument_registers: Default::default(),
                arguments: smallvec::smallvec![
                    receiver.clone(),
                    DexValue::String(std::rc::Rc::from(key)),
                    DexValue::String(std::rc::Rc::from(value)),
                ],
            };
            self.write_hash_map_value(&put)?;
        }

        match error {
            None => Some(DexExecutionResult::Void),
            Some(PropertiesLoadError::MalformedUnicodeEscape) => Some(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/IllegalArgumentException;".to_owned(),
                    detail_message: Some("Malformed \\uxxxx encoding.".to_owned()),
                }),
            )),
            Some(PropertiesLoadError::IndexOutOfBounds { length, index }) => {
                Some(DexExecutionResult::Throw(
                    self.allocate_generated_exception(DexGeneratedException {
                        class_descriptor: "Ljava/lang/ArrayIndexOutOfBoundsException;".to_owned(),
                        detail_message: Some(format!("length={length}; index={index}")),
                    }),
                ))
            }
        }
    }
}
