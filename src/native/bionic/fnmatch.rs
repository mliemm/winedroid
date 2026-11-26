use super::ctype::{wide_class_member, wide_class_of};
use super::export::in_libc;
use super::guest_mem::cstr_bytes;

// cite: bionic fnmatch.h#FNM_NOMATCH, FNM_NOESCAPE, FNM_PATHNAME, FNM_PERIOD, FNM_LEADING_DIR, FNM_CASEFOLD
const FNM_NOMATCH: i32 = 1;
const FNM_NOESCAPE: i32 = 0x01;
const FNM_PATHNAME: i32 = 0x02;
const FNM_PERIOD: i32 = 0x04;
const FNM_LEADING_DIR: i32 = 0x08;
const FNM_CASEFOLD: i32 = 0x10;

fn byte(text: &[u8], index: usize) -> u8 {
    text.get(index).copied().unwrap_or(0)
}

fn same(left: u8, right: u8, fold_case: bool) -> bool {
    left == right || (fold_case && left.eq_ignore_ascii_case(&right))
}

fn in_range(test: u8, start: u8, end: u8, fold_case: bool) -> bool {
    let exact = (start..=end).contains(&test);
    let any_upper = [test, start, end].iter().any(u8::is_ascii_uppercase);
    let folded = start.to_ascii_lowercase()..=end.to_ascii_lowercase();
    exact || (fold_case && any_upper && folded.contains(&test.to_ascii_lowercase()))
}

fn class_match(pattern: &[u8], position: &mut usize, test: u8, fold_case: bool) -> Option<bool> {
    // cite: bionic fnmatch.c#classmatch
    // cite: bionic charclass.h#cclasses
    if byte(pattern, *position) != b'[' || byte(pattern, *position + 1) != b':' {
        return None;
    }
    let rest = &pattern[*position + 2..];
    let length = rest.iter().position(|&character| character == b':')?;
    if byte(rest, length + 1) != b']' {
        return None;
    }
    let name: &[u8] = if fold_case && rest.starts_with(b"upper:]") {
        b"lower"
    } else {
        &rest[..length]
    };
    let class = wide_class_of(name);
    if class == 0 {
        return None;
    }
    *position += 2 + length + 2;
    Some(wide_class_member(class, i32::from(test)) != 0)
}

fn match_one(pattern: &[u8], p: &mut usize, string: &[u8], s: &mut usize, flags: i32) -> bool {
    // cite: bionic fnmatch.c#fnmatch_ch
    let mismatch = *p;
    let fold_case = flags & FNM_CASEFOLD != 0;
    let escape = flags & FNM_NOESCAPE == 0;
    let slash = flags & FNM_PATHNAME != 0;
    let test = byte(string, *s);
    if byte(pattern, *p) == b'[' {
        *p += 1;
        let negate = matches!(byte(pattern, *p), b'!' | b'^');
        if negate {
            *p += 1;
        }
        let mut leading_close = byte(pattern, *p) == b']';
        let mut found = false;
        while byte(pattern, *p) != 0 {
            if !leading_close {
                if byte(pattern, *p) == b']' {
                    *p += 1;
                    *s += 1;
                    return found != negate;
                }
                if escape && byte(pattern, *p) == b'\\' {
                    *p += 1;
                    if byte(pattern, *p) == 0 {
                        break;
                    }
                }
                if slash && byte(pattern, *p) == b'/' {
                    break;
                }
                if let Some(member) = class_match(pattern, p, test, fold_case) {
                    found |= member;
                    continue;
                }
            }
            leading_close = false;
            if byte(pattern, *p + 1) == b'-' && byte(pattern, *p + 2) != b']' {
                let start = byte(pattern, *p);
                *p += if escape && byte(pattern, *p + 2) == b'\\' {
                    3
                } else {
                    2
                };
                let end = byte(pattern, *p);
                if end == 0 || (slash && end == b'/') {
                    break;
                }
                found |= in_range(test, start, end, fold_case);
            } else {
                found |= same(test, byte(pattern, *p), fold_case);
            }
            *p += 1;
        }
        *p = mismatch;
    } else if byte(pattern, *p) == b'?' {
        if test == 0 || (slash && test == b'/') {
            return false;
        }
        *p += 1;
        *s += 1;
        return true;
    } else if escape && byte(pattern, *p) == b'\\' && byte(pattern, *p + 1) != 0 {
        *p += 1;
    }
    let expected = byte(pattern, *p);
    let matched = same(test, expected, fold_case);
    if test == 0 || expected == 0 || (slash && (test == b'/' || expected == b'/')) {
        return matched;
    }
    *p += 1;
    *s += 1;
    matched
}

fn fnmatch(pattern: &[u8], string: &[u8], flags: i32) -> i32 {
    // cite: bionic fnmatch.c#fnmatch
    let escape = flags & FNM_NOESCAPE == 0;
    let slash = flags & FNM_PATHNAME != 0;
    let leading_dir = flags & FNM_LEADING_DIR != 0;
    let escaped = |p: usize, character: u8| {
        escape && byte(pattern, p) == b'\\' && byte(pattern, p + 1) == character
    };
    let separator = |p: usize| slash && (byte(pattern, p) == b'/' || escaped(p, b'/'));
    let (mut p, mut s) = (0, 0);
    let (mut star_p, mut star_s, mut fixed) = (0, 0, 0);
    let mut first_star = byte(pattern, 0) == b'*';
    while first_star || (byte(pattern, p) != 0 && byte(string, s) != 0) {
        first_star = false;
        if slash && escaped(p, b'/') {
            p += 1;
        }
        if slash && byte(pattern, p) == b'/' && byte(string, s) == b'/' {
            p += 1;
            s += 1;
        }
        if flags & FNM_PERIOD != 0 && byte(string, s) == b'.' {
            if byte(pattern, p) == b'.' {
                p += 1;
            } else if escaped(p, b'.') {
                p += 2;
            } else {
                return FNM_NOMATCH;
            }
            s += 1;
        }
        let segment_end = string[s..]
            .iter()
            .position(|&character| slash && character == b'/')
            .map_or(string.len(), |offset| s + offset);
        while byte(pattern, p) != 0 {
            if s > segment_end || (s == segment_end && byte(pattern, p) != b'*') {
                break;
            }
            if separator(p) {
                break;
            }
            let mut wild = false;
            while matches!(byte(pattern, p), b'*' | b'?') {
                if byte(pattern, p) == b'*' {
                    wild = true;
                } else if s < segment_end {
                    s += 1;
                } else {
                    return FNM_NOMATCH;
                }
                p += 1;
            }
            if wild {
                star_s = s;
                star_p = p;
                let mut ahead = p;
                fixed = 0;
                loop {
                    if byte(pattern, ahead) == 0 || separator(ahead) {
                        if s + fixed > segment_end {
                            return FNM_NOMATCH;
                        }
                        s = segment_end - fixed;
                        wild = false;
                        break;
                    }
                    if byte(pattern, ahead) == b'*' {
                        if s + fixed > segment_end {
                            return FNM_NOMATCH;
                        }
                        break;
                    }
                    if escape && byte(pattern, ahead) == b'\\' && byte(pattern, ahead + 1) != 0 {
                        ahead += 2;
                    } else if byte(pattern, ahead) == b'[' {
                        match_one(pattern, &mut ahead, b" ", &mut 0, flags);
                    } else {
                        ahead += 1;
                    }
                    fixed += 1;
                }
            }
            while byte(pattern, p) != 0 && s < segment_end {
                if byte(pattern, p) == b'*' {
                    break;
                }
                if (slash && byte(string, s) == b'/') || separator(p) {
                    break;
                }
                if match_one(pattern, &mut p, string, &mut s, flags) {
                    continue;
                }
                if !wild {
                    return FNM_NOMATCH;
                }
                star_s += 1;
                s = star_s;
                if s + fixed > segment_end {
                    return FNM_NOMATCH;
                }
                p = star_p;
            }
        }
        if byte(string, s) != 0 && !((slash || leading_dir) && byte(string, s) == b'/') {
            return FNM_NOMATCH;
        }
        if byte(pattern, p) != 0 && !separator(p) {
            return FNM_NOMATCH;
        }
        if leading_dir && byte(pattern, p) == 0 && byte(string, s) == b'/' {
            return 0;
        }
    }
    if byte(string, s) == 0 && byte(pattern, p) == 0 {
        0
    } else {
        FNM_NOMATCH
    }
}

pub fn register() {
    in_libc("fnmatch", |context| {
        let result = fnmatch(
            cstr_bytes(context.arg_ptr(0)),
            cstr_bytes(context.arg_ptr(1)),
            context.arg_i32(2),
        );
        context.ret_i32(result);
    });
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;
    use crate::native::{
        bridge::BridgeProvider, call_assign::CallBuilder, handler_registry::register_all_groups,
        link_resolution::SystemLibraryProvider,
    };

    fn guest_fnmatch(pattern: &str, string: &[u8], flags: i32) -> i32 {
        register_all_groups();
        let address = BridgeProvider.export("libc.so", "fnmatch").unwrap();
        let pattern = CString::new(pattern).unwrap();
        let string = CString::new(string).unwrap();
        CallBuilder::new(address)
            .ptr(pattern.as_ptr() as usize)
            .ptr(string.as_ptr() as usize)
            .i32(flags)
            .invoke()
            .unwrap()
            .i32()
    }

    fn check(cases: &[(&str, &str, i32, i32)]) {
        for &(pattern, string, flags, expected) in cases {
            assert_eq!(
                guest_fnmatch(pattern, string.as_bytes(), flags),
                expected,
                "fnmatch({pattern:?}, {string:?}, {flags:#x})"
            );
        }
    }

    #[test]
    fn upstream_openbsd_fnmatch_header_comment_rules_hold_through_the_libc_export() {
        // cite: bionic fnmatch.c#fnmatch, fnmatch_ch
        check(&[
            ("[^a]", "b", 0, 0),
            ("[^a]", "a", 0, 1),
            ("[!a]", "a", 0, 1),
            ("[.abc]x", ".x", FNM_PERIOD, 1),
            ("a[b", "a[b", 0, 0),
            ("a//b", "a/b", FNM_PATHNAME, 1),
            ("a\\", "a\\", 0, 0),
            ("[a-m-z]", "c", 0, 0),
            ("[a-m-z]", "-", 0, 0),
            ("[a-m-z]", "z", 0, 0),
            ("[a-m-z]", "n", 0, 1),
            ("[]a]", "]", 0, 0),
        ]);
    }

    #[test]
    fn bionic_fnmatch_h_flag_values_select_the_documented_behaviour() {
        // cite: bionic fnmatch.h#FNM_NOESCAPE, FNM_PATHNAME, FNM_PERIOD, FNM_LEADING_DIR, FNM_CASEFOLD
        check(&[
            ("*.so", "libz.so", 0, 0),
            ("*.so", "libz.so.1", 0, 1),
            ("a*b", "a/xb", 0, 0),
            ("a*b", "a/xb", 0x02, 1),
            ("a?b", "a/b", 0x02, 1),
            ("*/*.c", "src/a.c", 0x02, 0),
            ("*", ".hidden", 0x04, 1),
            (".*", ".hidden", 0x04, 0),
            ("a/*", "a/.x", 0x06, 1),
            ("/data/*", "/data/app/base.apk", 0x02, 1),
            ("/data/*", "/data/app/base.apk", 0x0a, 0),
            ("LIB*", "libc", 0, 1),
            ("LIB*", "libc", 0x10, 0),
            ("a\\*", "a*", 0, 0),
            ("a\\*", "ab", 0, 1),
            ("a\\*", "a\\bc", 0x01, 0),
        ]);
    }

    #[test]
    fn upstream_openbsd_charclass_names_and_unsigned_arm64_bytes_are_honoured() {
        // cite: bionic charclass.h#cclasses
        // cite: bionic fnmatch.c#classmatch, fnmatch_ch
        check(&[
            ("[[:digit:]]*", "7z", 0, 0),
            ("[[:digit:]]*", "z7", 0, 1),
            ("*[[:space:]]?", "a b", 0, 0),
            ("[[:upper:]]", "a", 0, 1),
            ("[[:upper:]]", "a", FNM_CASEFOLD, 0),
            ("[[:nope:]]", "n]", 0, 0),
            ("[A-C]", "b", FNM_CASEFOLD, 0),
        ]);
        assert_eq!(guest_fnmatch("[\u{1}-é]", b"\x7f", 0), 0);
    }
}
