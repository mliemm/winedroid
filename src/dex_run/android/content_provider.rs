use super::lru_cache::guest_hook_method;
use super::*;

const CONTENT_PROVIDER: &str = "Landroid/content/ContentProvider;";
const CONTENT_RESOLVER: &str = "Landroid/content/ContentResolver;";
const CONTEXT: &str = "Landroid/content/Context;";
const PROVIDER_INFO: &str = "Landroid/content/pm/ProviderInfo;";
const URI: &str = "Landroid/net/Uri;";
const STRING_ARRAY: &str = "[Ljava/lang/String;";
const STRING: &str = "Ljava/lang/String;";
const CURSOR: &str = "Landroid/database/Cursor;";
const BUNDLE: &str = "Landroid/os/Bundle;";
const CANCELLATION_SIGNAL: &str = "Landroid/os/CancellationSignal;";
const SCHEME_CONTENT: &str = "content";

const QUERY_DESCRIPTOR: &str = "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;";

fn provider_context_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: CONTENT_PROVIDER.to_owned(),
        type_descriptor: "Landroid/content/Context;".to_owned(),
        name: "mContext".to_owned(),
    }
}

fn provider_calling_package_field() -> DexFieldId {
    DexFieldId {
        class_descriptor: CONTENT_PROVIDER.to_owned(),
        type_descriptor: STRING.to_owned(),
        name: "mCallingPackage".to_owned(),
    }
}

fn is_null(value: &DexValue) -> bool {
    matches!(value, DexValue::Null | DexValue::Int(0))
}

impl DexProgram {
    pub(in crate::dex_run) fn invoke_content_provider_method(
        &self,
        method: &DexMethodId,
        invocation: &DexInvocation,
        depth: usize,
    ) -> Result<Option<DexExecutionResult>> {
        let descriptor = method.proto.descriptor();
        let arguments = invocation.arguments.as_slice();
        match (
            method.class_descriptor.as_str(),
            method.name.as_str(),
            descriptor.as_str(),
            arguments,
        ) {
            // cite: frameworks-base ContentProvider.java#ContentProvider
            (CONTENT_PROVIDER, "<init>", "()V", [_]) => Ok(Some(DexExecutionResult::Void)),
            // cite: frameworks-base ProviderInfo.java#ProviderInfo
            // cite: frameworks-base ComponentInfo.java#enabled
            (PROVIDER_INFO, "<init>", "()V", [info]) => {
                self.write_stored_instance_field(
                    info,
                    &DexFieldId {
                        class_descriptor: PROVIDER_INFO.to_owned(),
                        type_descriptor: "Z".to_owned(),
                        name: "enabled".to_owned(),
                    },
                    DexValue::Int(1),
                );

                Ok(Some(DexExecutionResult::Void))
            }
            // cite: frameworks-base ContentProvider.java#getCallingPackage, mCallingAttributionSource
            (CONTENT_PROVIDER, "getCallingPackage", "()Ljava/lang/String;", [provider]) => {
                let package = self
                    .read_stored_instance_field(provider, &provider_calling_package_field())
                    .unwrap_or(DexValue::Null);

                Ok(Some(DexExecutionResult::Value(package)))
            }
            // cite: frameworks-base ContentProvider.java#getContext, mContext
            (CONTENT_PROVIDER, "getContext", "()Landroid/content/Context;", [provider]) => {
                let context = self
                    .read_stored_instance_field(provider, &provider_context_field())
                    .unwrap_or(DexValue::Null);

                Ok(Some(DexExecutionResult::Value(context)))
            }
            // cite: frameworks-base ContentProvider.java#attachInfo
            (
                CONTENT_PROVIDER,
                "attachInfo",
                "(Landroid/content/Context;Landroid/content/pm/ProviderInfo;)V",
                [provider, context, _],
            ) => Ok(Some(
                match self.content_provider_attach_info(provider, context, depth)? {
                    Some(exception) => DexExecutionResult::Throw(exception),
                    None => DexExecutionResult::Void,
                },
            )),
            // cite: frameworks-base ContentResolver.java#query
            (
                CONTENT_RESOLVER,
                "query",
                QUERY_DESCRIPTOR,
                [_, uri, projection, selection, selection_args, sort_order],
            ) => self
                .content_resolver_query(
                    uri,
                    [projection, selection, selection_args, sort_order],
                    depth,
                )
                .map(Some),
            _ => Ok(None),
        }
    }

    // cite: frameworks-base ContentResolver.java#query, acquireUnstableProvider
    fn content_resolver_query(
        &self,
        uri: &DexValue,
        query_arguments: [&DexValue; 4],
        depth: usize,
    ) -> Result<DexExecutionResult> {
        if is_null(uri) {
            return Ok(DexExecutionResult::Throw(
                self.allocate_generated_exception(DexGeneratedException {
                    class_descriptor: "Ljava/lang/NullPointerException;".to_owned(),
                    detail_message: Some("uri".to_owned()),
                }),
            ));
        }
        let (Some(scheme), Some(authority)) = (
            self.uri_value_component(uri, "scheme"),
            self.uri_value_component(uri, "authority"),
        ) else {
            return Err(WinedroidError::new(format!(
                "ContentResolver.query requires a string backed Uri\nuri: {uri}\nnext missing primitive: ContentResolver.query with an opaque Uri"
            )));
        };
        let DexValue::String(authority) = authority else {
            return Ok(DexExecutionResult::Value(DexValue::Null));
        };
        if scheme != DexValue::String(std::rc::Rc::from(SCHEME_CONTENT)) {
            return Ok(DexExecutionResult::Value(DexValue::Null));
        }

        let provider = match self.acquire_content_provider(&authority, depth)? {
            Ok(Some(provider)) => provider,
            Ok(None) => return Ok(DexExecutionResult::Value(DexValue::Null)),
            Err(exception) => return Ok(DexExecutionResult::Throw(exception)),
        };
        self.content_provider_query(&provider, uri, query_arguments, depth)
    }

    // cite: frameworks-base ContentProvider.java#query
    fn content_provider_query(
        &self,
        provider: &DexValue,
        uri: &DexValue,
        query_arguments: [&DexValue; 4],
        depth: usize,
    ) -> Result<DexExecutionResult> {
        let class_descriptor = value_class_descriptor(provider).unwrap_or(CONTENT_PROVIDER);
        let wider_overrides = [
            (
                "query",
                vec![URI, STRING_ARRAY, BUNDLE, CANCELLATION_SIGNAL],
            ),
            (
                "query",
                vec![
                    URI,
                    STRING_ARRAY,
                    STRING,
                    STRING_ARRAY,
                    STRING,
                    CANCELLATION_SIGNAL,
                ],
            ),
        ];
        for (name, parameters) in wider_overrides {
            let hook = guest_hook_method(class_descriptor, name, CURSOR, &parameters);
            if self
                .resolve_exact_method_in_class_hierarchy(class_descriptor, &hook)?
                .is_some()
            {
                return Err(WinedroidError::new(format!(
                    "ContentProvider.query dispatch needs the {} override\nprovider: {class_descriptor}\nnext missing primitive: ContentProvider.query override ({})",
                    hook.display_signature(),
                    parameters.join("")
                )));
            }
        }
        let mut arguments = vec![uri.clone()];
        arguments.extend(query_arguments.into_iter().cloned());
        // cite: frameworks-base ContentProvider.java#Transport, setCallingAttributionSource
        let caller = self.package_name.as_deref().map_or(DexValue::Null, |name| {
            DexValue::String(std::rc::Rc::from(name))
        });
        let original = self
            .read_stored_instance_field(provider, &provider_calling_package_field())
            .unwrap_or(DexValue::Null);
        self.write_stored_instance_field(provider, &provider_calling_package_field(), caller);
        let result = self.guest_override(
            provider,
            "query",
            CURSOR,
            &[URI, STRING_ARRAY, STRING, STRING_ARRAY, STRING],
            arguments,
            depth,
        );
        self.write_stored_instance_field(provider, &provider_calling_package_field(), original);
        match result? {
            Some(result) => Ok(result),
            None => Err(WinedroidError::new(format!(
                "ContentProvider.query is abstract and the provider declares no override\nprovider: {class_descriptor}\nnext missing primitive: ContentProvider.query {QUERY_DESCRIPTOR}"
            ))),
        }
    }

    // cite: frameworks-base ActivityThread.java#acquireProvider, installProvider
    // cite: frameworks-base ContentProviderHolder.java#ContentProviderHolder
    fn acquire_content_provider(
        &self,
        authority: &str,
        depth: usize,
    ) -> Result<std::result::Result<Option<DexValue>, DexValue>> {
        if let Some(provider) = self.runtime.borrow().content_providers.get(authority) {
            return Ok(Ok(Some(provider.clone())));
        }
        // cite: frameworks-base ActivityThread.java#installProviderAuthoritiesLocked, ProviderKey
        let Some(manifest_provider) = self
            .manifest_providers
            .iter()
            .find(|provider| provider.authority.split(';').any(|name| name == authority))
        else {
            return Err(WinedroidError::new(format!(
                "ContentResolver has no provider table for authority {authority}\nnext missing primitive: ContentResolver provider lookup for a non manifest authority"
            )));
        };
        if !manifest_provider.is_enabled() {
            return Ok(Ok(None));
        }

        let descriptor = normalize_class_name(&manifest_provider.name);
        let class_reference =
            DexValue::Reference(std::rc::Rc::from(format!("Ljava/lang/Class;:{descriptor}")));
        let new_instance = DexInvocation {
            kind: DexInvokeKind::Static,
            range: false,
            method_index: 0,
            argument_registers: std::iter::once(0).collect(),
            arguments: std::iter::once(class_reference).collect(),
        };
        let provider = match self.class_new_instance(&new_instance, None, depth + 1)? {
            DexExecutionResult::Value(provider) => provider,
            DexExecutionResult::Throw(exception) => return Ok(Err(exception)),
            DexExecutionResult::Parked(execution) => {
                self.discard_parked_execution(&execution);
                return Err(WinedroidError::new(format!(
                    "ContentProvider instantiation did not complete normally\nprovider: {descriptor}"
                )));
            }
            DexExecutionResult::Void | DexExecutionResult::Int(_) => {
                return Err(WinedroidError::new(format!(
                    "ContentProvider instantiation returned no instance\nprovider: {descriptor}"
                )))
            }
        };

        let package_name = self.package_name.clone().unwrap_or_default();
        let application_info = DexValue::Reference(std::rc::Rc::from(
            "Landroid/content/pm/ApplicationInfo;.bootstrap",
        ));
        let info = self.provider_info_value(&package_name, &application_info, manifest_provider);
        if let Some(exception) = self.install_provider_attach_info(&provider, &info, depth)? {
            return Ok(Err(exception));
        }
        let mut runtime = self.runtime.borrow_mut();
        for name in manifest_provider.authority.split(';') {
            runtime
                .content_providers
                .insert(name.to_owned(), provider.clone());
        }

        Ok(Ok(Some(provider)))
    }

    // cite: frameworks-base ActivityThread.java#installProvider
    fn install_provider_attach_info(
        &self,
        provider: &DexValue,
        info: &DexValue,
        depth: usize,
    ) -> Result<Option<DexValue>> {
        let context = self.application_value();
        match self.guest_override(
            provider,
            "attachInfo",
            "V",
            &[CONTEXT, PROVIDER_INFO],
            vec![context.clone(), info.clone()],
            depth,
        )? {
            Some(DexExecutionResult::Throw(exception)) => Ok(Some(exception)),
            Some(_) => Ok(None),
            None => self.content_provider_attach_info(provider, &context, depth),
        }
    }

    // cite: frameworks-base ContentProvider.java#attachInfo, mContext
    fn content_provider_attach_info(
        &self,
        provider: &DexValue,
        context: &DexValue,
        depth: usize,
    ) -> Result<Option<DexValue>> {
        if let Some(existing) = self.read_stored_instance_field(provider, &provider_context_field())
        {
            if !is_null(&existing) {
                return Ok(None);
            }
        }
        self.write_stored_instance_field(provider, &provider_context_field(), context.clone());
        match self.guest_override(provider, "onCreate", "Z", &[], Vec::new(), depth)? {
            Some(DexExecutionResult::Throw(exception)) => Ok(Some(exception)),
            Some(_) => Ok(None),
            None => Err(WinedroidError::new(format!(
                "ContentProvider.onCreate is abstract and the provider declares no override\nprovider: {provider}\nnext missing primitive: ContentProvider.onCreate"
            ))),
        }
    }
}
