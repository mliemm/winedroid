use super::*;

const URI: &str = "Landroid/net/Uri;";
const URI_BUILDER: &str = "Landroid/net/Uri$Builder;";

#[derive(Default)]
struct UriParts {
    scheme: Option<String>,
    authority: Option<String>,
    path: Option<String>,
    query: Option<String>,
    fragment: Option<String>,
}

const BUILDER_PARTS: [&str; 5] = ["scheme", "authority", "path", "query", "fragment"];

// cite: frameworks-base Uri.java#isAllowed, encode
fn uri_encode(text: &str) -> String {
    let mut encoded = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() || "_-!.~'()*".contains(character) {
            encoded.push(character);
            continue;
        }
        let mut buffer = [0u8; 4];
        for byte in character.encode_utf8(&mut buffer).bytes() {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    encoded
}

// cite: frameworks-base Uri.java#StringUri, isHierarchical, parseScheme, parseAuthority, parsePath, parseQuery, parseFragment
fn parse_hierarchical_string_uri(uri: &str) -> Option<UriParts> {
    let length = uri.len() as isize;
    let bytes = uri.as_bytes();
    let ssi = uri.find(':').map_or(-1, |index| index as isize);
    if ssi != -1 && (length == ssi + 1 || bytes[(ssi + 1) as usize] != b'/') {
        return None;
    }
    let byte_at = |index: isize| bytes[index as usize];
    let has_authority = length > ssi + 2 && byte_at(ssi + 1) == b'/' && byte_at(ssi + 2) == b'/';

    let authority = if has_authority {
        let mut end = ssi + 3;
        while end < length && !matches!(byte_at(end), b'/' | b'\\' | b'?' | b'#') {
            end += 1;
        }
        Some(uri[(ssi + 3) as usize..end as usize].to_owned())
    } else {
        None
    };

    let mut path_start = ssi + 1;
    let mut empty_path = false;
    if has_authority {
        path_start = ssi + 3;
        while path_start < length {
            match byte_at(path_start) {
                b'?' | b'#' => {
                    empty_path = true;
                    break;
                }
                b'/' | b'\\' => break,
                _ => path_start += 1,
            }
        }
    }
    let path = if empty_path {
        String::new()
    } else {
        let mut path_end = path_start;
        while path_end < length && !matches!(byte_at(path_end), b'?' | b'#') {
            path_end += 1;
        }
        uri[path_start as usize..path_end as usize].to_owned()
    };

    let fragment_start = uri[(ssi.max(0)) as usize..]
        .find('#')
        .map(|index| index + ssi.max(0) as usize);
    let query = match uri[(ssi.max(0)) as usize..]
        .find('?')
        .map(|index| index + ssi.max(0) as usize)
    {
        None => None,
        Some(query_start) => match fragment_start {
            None => Some(uri[query_start + 1..].to_owned()),
            Some(fragment_start) if fragment_start < query_start => None,
            Some(fragment_start) => Some(uri[query_start + 1..fragment_start].to_owned()),
        },
    };

    Some(UriParts {
        scheme: (ssi != -1).then(|| uri[..ssi as usize].to_owned()),
        authority,
        path: Some(path),
        query,
        fragment: fragment_start.map(|start| uri[start + 1..].to_owned()),
    })
}

// cite: frameworks-base Uri.java#Builder, build, makeAbsolute, HierarchicalUri, appendSspTo, makeUriString
fn build_uri_string(parts: &UriParts) -> String {
    let mut text = String::new();
    if let Some(scheme) = &parts.scheme {
        text.push_str(scheme);
        text.push(':');
    }
    if let Some(authority) = &parts.authority {
        text.push_str("//");
        text.push_str(authority);
    }
    let mut path = parts.path.clone().unwrap_or_default();
    let has_scheme_or_authority = parts.scheme.is_some() || parts.authority.is_some();
    if has_scheme_or_authority && !path.is_empty() && !path.starts_with('/') {
        path.insert(0, '/');
    }
    text.push_str(&path);
    if let Some(query) = parts.query.as_deref().filter(|query| !query.is_empty()) {
        text.push('?');
        text.push_str(query);
    }
    if let Some(fragment) = parts
        .fragment
        .as_deref()
        .filter(|fragment| !fragment.is_empty())
    {
        text.push('#');
        text.push_str(fragment);
    }

    text
}

fn builder_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: URI_BUILDER.to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: name.to_owned(),
    }
}

fn optional_string(value: &Option<String>) -> DexValue {
    match value {
        Some(text) => DexValue::String(std::rc::Rc::from(text.as_str())),
        None => DexValue::Null,
    }
}

impl DexProgram {
    pub(in crate::dex_run) fn invoke_uri_builder_method(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Result<Option<DexExecutionResult>> {
        let descriptor = method.proto.descriptor();
        let arguments = invocation.arguments.as_slice();
        let value = match (
            method.class_descriptor.as_str(),
            method.name.as_str(),
            descriptor.as_str(),
            arguments,
        ) {
            // cite: frameworks-base Uri.java#buildUpon
            (URI, "buildUpon", "()Landroid/net/Uri$Builder;", [uri]) => self.uri_build_upon(uri),
            // cite: frameworks-base Uri.java#appendPath, appendDecodedSegment, appendEncodedSegment
            (
                URI_BUILDER,
                "appendPath",
                "(Ljava/lang/String;)Landroid/net/Uri$Builder;",
                [builder, segment],
            ) => self.uri_builder_append_path(builder, segment),
            // cite: frameworks-base Uri.java#build
            (URI_BUILDER, "build", "()Landroid/net/Uri;", [builder]) => {
                self.uri_builder_build(builder)
            }
            _ => return Ok(None),
        };

        Ok(value.map(DexExecutionResult::Value))
    }

    fn uri_build_upon(&self, uri: &DexValue) -> Option<DexValue> {
        let parts = parse_hierarchical_string_uri(&self.uri_text(uri)?)?;
        let builder = self.allocate_object(URI_BUILDER.to_owned());
        self.write_builder_parts(&builder, &parts)
            .then_some(builder)
    }

    fn write_builder_parts(&self, builder: &DexValue, parts: &UriParts) -> bool {
        let values = [
            &parts.scheme,
            &parts.authority,
            &parts.path,
            &parts.query,
            &parts.fragment,
        ];
        BUILDER_PARTS.iter().zip(values).all(|(name, value)| {
            self.write_stored_instance_field(builder, &builder_field(name), optional_string(value))
        })
    }

    fn read_builder_parts(&self, builder: &DexValue) -> Option<UriParts> {
        let mut values = Vec::new();
        for name in BUILDER_PARTS {
            values.push(
                match self.read_stored_instance_field(builder, &builder_field(name))? {
                    DexValue::String(text) => Some(text.to_string()),
                    DexValue::Null => None,
                    _ => return None,
                },
            );
        }
        let mut values = values.into_iter();

        Some(UriParts {
            scheme: values.next()?,
            authority: values.next()?,
            path: values.next()?,
            query: values.next()?,
            fragment: values.next()?,
        })
    }

    fn uri_builder_append_path(&self, builder: &DexValue, segment: &DexValue) -> Option<DexValue> {
        let mut parts = self.read_builder_parts(builder)?;
        let segment = match segment {
            DexValue::String(text) => uri_encode(text),
            DexValue::Null => "null".to_owned(),
            _ => return None,
        };
        let old_path = parts.path.take().unwrap_or_default();
        parts.path = Some(if old_path.is_empty() {
            format!("/{segment}")
        } else if old_path.ends_with('/') {
            format!("{old_path}{segment}")
        } else {
            format!("{old_path}/{segment}")
        });
        self.write_builder_parts(builder, &parts)
            .then(|| builder.clone())
    }

    fn uri_builder_build(&self, builder: &DexValue) -> Option<DexValue> {
        let text = build_uri_string(&self.read_builder_parts(builder)?);
        let uri = self.allocate_object(URI.to_owned());
        self.write_stored_instance_field(
            &uri,
            &uri_string_field(),
            DexValue::String(std::rc::Rc::from(text)),
        )
        .then_some(uri)
    }
}
