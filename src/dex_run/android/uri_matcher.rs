use super::*;

const URI_MATCHER: &str = "Landroid/content/UriMatcher;";
const NO_MATCH: i32 = -1;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Which {
    Root,
    Exact,
    Number,
    Text,
}

pub(in crate::dex_run) struct UriMatcherNode {
    code: i32,
    which: Which,
    text: Option<String>,
    children: Vec<UriMatcherNode>,
}

impl UriMatcherNode {
    fn root(code: i32) -> Self {
        Self {
            code,
            which: Which::Root,
            text: None,
            children: Vec::new(),
        }
    }

    // cite: frameworks-base UriMatcher.java#createChild
    fn child(token: &str) -> Self {
        let which = match token {
            "#" => Which::Number,
            "*" => Which::Text,
            _ => Which::Exact,
        };

        Self {
            code: NO_MATCH,
            which,
            text: Some(token.to_owned()),
            children: Vec::new(),
        }
    }
}

// cite: libcore String.java#split
fn java_split_slash(text: &str) -> Vec<String> {
    if !text.contains('/') {
        return vec![text.to_owned()];
    }
    let mut tokens: Vec<String> = text.split('/').map(str::to_owned).collect();
    while tokens.last().is_some_and(String::is_empty) {
        tokens.pop();
    }

    tokens
}

impl DexProgram {
    pub(in crate::dex_run) fn invoke_uri_matcher_method(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
    ) -> Result<Option<DexExecutionResult>> {
        if method.class_descriptor != URI_MATCHER {
            return Ok(None);
        }
        let Some(receiver) = invocation.arguments.first() else {
            return Ok(None);
        };
        let arguments = &invocation.arguments[1..];
        let key = map_identity_key(receiver);
        let descriptor = method.proto.descriptor();
        match (method.name.as_str(), descriptor.as_str(), arguments) {
            // cite: frameworks-base UriMatcher.java#UriMatcher
            ("<init>", "(I)V", [DexValue::Int(code)]) => {
                self.runtime
                    .borrow_mut()
                    .uri_matchers
                    .insert(key, UriMatcherNode::root(*code));

                Ok(Some(DexExecutionResult::Void))
            }
            // cite: frameworks-base UriMatcher.java#addURI
            (
                "addURI",
                "(Ljava/lang/String;Ljava/lang/String;I)V",
                [authority, path, DexValue::Int(code)],
            ) => self.uri_matcher_add(&key, authority, path, *code).map(Some),
            // cite: frameworks-base UriMatcher.java#match
            ("match", "(Landroid/net/Uri;)I", [uri]) => self.uri_matcher_match(&key, uri).map(Some),
            _ => Ok(None),
        }
    }

    fn uri_matcher_exception(&self, class_descriptor: &str, message: Option<String>) -> DexValue {
        self.allocate_generated_exception(DexGeneratedException {
            class_descriptor: class_descriptor.to_owned(),
            detail_message: message,
        })
    }

    fn uri_matcher_add(
        &self,
        key: &str,
        authority: &DexValue,
        path: &DexValue,
        code: i32,
    ) -> Result<DexExecutionResult> {
        if code < 0 {
            return Ok(DexExecutionResult::Throw(self.uri_matcher_exception(
                "Ljava/lang/IllegalArgumentException;",
                Some(format!("code {code} is invalid: it must be positive")),
            )));
        }
        let tokens = match path {
            DexValue::Null => Vec::new(),
            DexValue::String(path) => {
                let stripped = match path.strip_prefix('/') {
                    Some(rest) if path.len() > 1 => rest,
                    _ => path.as_ref(),
                };
                java_split_slash(stripped)
            }
            _ => {
                return Err(WinedroidError::new(format!(
                    "UriMatcher.addURI path is not a String\npath: {path}\nnext missing primitive: UriMatcher.addURI with a non-String path"
                )))
            }
        };
        let DexValue::String(authority) = authority else {
            if matches!(authority, DexValue::Null) {
                return Ok(DexExecutionResult::Throw(
                    self.uri_matcher_exception("Ljava/lang/NullPointerException;", None),
                ));
            }
            return Err(WinedroidError::new(format!(
                "UriMatcher.addURI authority is not a String\nauthority: {authority}\nnext missing primitive: UriMatcher.addURI with a non-String authority"
            )));
        };

        let mut runtime = self.runtime.borrow_mut();
        let Some(root) = runtime.uri_matchers.get_mut(key) else {
            return Err(WinedroidError::new(
                "UriMatcher used before <init>\nnext missing primitive: UriMatcher.<init> on this receiver",
            ));
        };
        let mut node = root;
        for token in std::iter::once(authority.to_string()).chain(tokens) {
            let position = match node
                .children
                .iter()
                .position(|child| child.text.as_deref() == Some(token.as_str()))
            {
                Some(position) => position,
                None => {
                    node.children.push(UriMatcherNode::child(&token));
                    node.children.len() - 1
                }
            };
            node = &mut node.children[position];
        }
        node.code = code;

        Ok(DexExecutionResult::Void)
    }

    fn uri_matcher_match(&self, key: &str, uri: &DexValue) -> Result<DexExecutionResult> {
        if matches!(uri, DexValue::Null) {
            return Ok(DexExecutionResult::Throw(
                self.uri_matcher_exception("Ljava/lang/NullPointerException;", None),
            ));
        }
        let Some(DexValue::String(path)) = self.uri_value_component(uri, "path") else {
            return Err(WinedroidError::new(format!(
                "UriMatcher.match could not read the Uri path\nuri: {uri}\nnext missing primitive: UriMatcher.match on this Uri value"
            )));
        };
        let authority = match self.uri_value_component(uri, "authority") {
            Some(DexValue::String(authority)) => Some(authority.to_string()),
            Some(DexValue::Null) => None,
            _ => {
                return Err(WinedroidError::new(format!(
                    "UriMatcher.match could not read the Uri authority\nuri: {uri}\nnext missing primitive: UriMatcher.match on this Uri value"
                )))
            }
        };
        let segments: Vec<&str> = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();

        let runtime = self.runtime.borrow();
        let Some(root) = runtime.uri_matchers.get(key) else {
            return Err(WinedroidError::new(
                "UriMatcher used before <init>\nnext missing primitive: UriMatcher.<init> on this receiver",
            ));
        };
        if segments.is_empty() && authority.is_none() {
            return Ok(DexExecutionResult::Int(root.code));
        }

        let mut node = root;
        let texts = std::iter::once(authority.as_deref()).chain(segments.into_iter().map(Some));
        for text in texts {
            let mut matched = None;
            for candidate in &node.children {
                let hit =
                    match candidate.which {
                        Which::Exact => candidate.text.as_deref() == text,
                        Which::Number => {
                            let Some(text) = text else {
                                drop(runtime);
                                return Ok(DexExecutionResult::Throw(self.uri_matcher_exception(
                                    "Ljava/lang/NullPointerException;",
                                    None,
                                )));
                            };
                            text.chars().all(|character| character.is_ascii_digit())
                        }
                        Which::Text => true,
                        Which::Root => false,
                    };
                if hit {
                    matched = Some(candidate);
                    break;
                }
            }
            let Some(next) = matched else {
                return Ok(DexExecutionResult::Int(NO_MATCH));
            };
            node = next;
        }

        Ok(DexExecutionResult::Int(node.code))
    }
}
