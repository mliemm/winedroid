use std::fs;
use std::path::PathBuf;

const SLOT_COMMENT: &str = concat!('/', '/');

#[derive(Debug, PartialEq, Eq)]
pub struct Slot {
    pub name: String,
    pub signature: String,
}

pub fn manifest_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

pub fn read_header() -> String {
    fs::read_to_string(manifest_path("tests/native_fixtures/include/jni.h"))
        .expect("jni.h should be readable")
}

fn normalize_parameter(parameter: &str) -> String {
    let spaced = parameter
        .replace("ATTRIBUTE_UNUSED", "")
        .replace('*', " * ");
    let mut tokens: Vec<&str> = spaced.split_whitespace().collect();
    let is_name = |token: &str| {
        token
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
    };
    if tokens.len() > 1
        && tokens
            .last()
            .is_some_and(|last| is_name(last) && *last != "const")
    {
        tokens.pop();
    }
    tokens.concat()
}

fn signature(return_type: &str, parameters: &str) -> String {
    let parameters: Vec<String> = parameters.split(',').map(normalize_parameter).collect();
    format!(
        "{}({})",
        normalize_parameter(&format!("{return_type} _")),
        parameters.join(",")
    )
}

pub fn header_slots(header: &str, struct_name: &str) -> Vec<Slot> {
    let start = header
        .find(&format!("struct {struct_name} {{"))
        .unwrap_or_else(|| panic!("jni.h has no struct {struct_name}"));
    let body = &header[start..];
    let body = &body[..body.find("\n};").expect("struct is not closed")];
    body.split(';')
        .filter_map(|statement| {
            let statement = statement.trim();
            if let Some(open) = statement.find("(*") {
                let name_end = statement[open..].find(')')? + open;
                let parameters = statement[name_end..]
                    .strip_prefix(")(")?
                    .trim_end_matches(')');
                return Some(Slot {
                    name: statement[open + 2..name_end].to_owned(),
                    signature: signature(&statement[..open], parameters),
                });
            }
            let name = statement.rsplit("void*").next()?.trim();
            (statement.contains("void*") && name.starts_with("reserved")).then(|| Slot {
                name: name.to_owned(),
                signature: String::new(),
            })
        })
        .collect()
}

fn art_definition(source: &str, name: &str) -> Option<String> {
    let at = source.lines().scan(0usize, |offset, line| {
        let start = *offset;
        *offset += line.len() + 1;
        Some((start, line))
    });
    for (start, line) in at {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("static ") else {
            continue;
        };
        let Some(open) = rest.find(&format!("{name}(")) else {
            continue;
        };
        let before = &rest[..open];
        if !(before.ends_with(' ') || before.ends_with('*')) {
            continue;
        }
        let call = start + (line.len() - rest.len()) + open + name.len();
        let mut depth = 0;
        for (index, character) in source[call..].char_indices() {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(signature(before, &source[call + 1..call + index]));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

pub fn art_slots(source: &str, table_marker: &str, prefix: &str) -> Vec<Slot> {
    let start = source
        .find(table_marker)
        .unwrap_or_else(|| panic!("art source has no {table_marker}"));
    let body = &source[start..];
    let body = &body[..body.find("};").expect("table is not closed")];
    let mut reserved = 0;
    body.lines()
        .skip(1)
        .filter_map(|line| {
            let entry = line
                .trim()
                .split(SLOT_COMMENT)
                .next()?
                .trim()
                .trim_end_matches(',');
            if entry == "nullptr" {
                reserved += 1;
                return Some(Slot {
                    name: format!("reserved{}", reserved - 1),
                    signature: String::new(),
                });
            }
            let name = entry.strip_prefix(prefix)?;
            let signature = art_definition(source, name)
                .unwrap_or_else(|| panic!("art source has no definition of {name}"));
            Some(Slot {
                name: name.to_owned(),
                signature,
            })
        })
        .collect()
}
