use super::super::*;

const LOCALE_REFERENCE_PREFIX: &str = "Ljava/util/Locale;:";

const LEGACY_LANGUAGE_TAGS: [&str; 26] = [
    "art-lojban",
    "cel-gaulish",
    "en-gb-oed",
    "i-ami",
    "i-bnn",
    "i-default",
    "i-enochian",
    "i-hak",
    "i-klingon",
    "i-lux",
    "i-mingo",
    "i-navajo",
    "i-pwn",
    "i-tao",
    "i-tay",
    "i-tsu",
    "no-bok",
    "no-nyn",
    "sgn-be-fr",
    "sgn-be-nl",
    "sgn-ch-de",
    "zh-guoyu",
    "zh-hakka",
    "zh-min",
    "zh-min-nan",
    "zh-xiang",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LocaleAccessor {
    Language,
    Country,
    Variant,
}

#[derive(Default, PartialEq, Eq)]
pub(in crate::dex_run) struct LocaleParts {
    pub(in crate::dex_run) language: String,
    pub(in crate::dex_run) script: String,
    pub(in crate::dex_run) region: String,
    pub(in crate::dex_run) variant: String,
}

fn locale_field(name: &str) -> DexFieldId {
    DexFieldId {
        class_descriptor: "Ljava/util/Locale;".to_owned(),
        type_descriptor: "Ljava/lang/String;".to_owned(),
        name: name.to_owned(),
    }
}

// cite: libcore Locale.java#Locale
// cite: libcore BaseLocale.java#getInstance
impl DexProgram {
    pub(super) fn initialize_locale(
        &self,
        invocation: &DexInvocation,
    ) -> Option<DexExecutionResult> {
        let (locale, language, country, variant) = match invocation.arguments.as_slice() {
            [locale, language, country] => {
                (locale, language, country, &DexValue::String("".into()))
            }
            [locale, language, country, variant] => (locale, language, country, variant),
            _ => return None,
        };
        if !dex_value_is_object_like(locale) {
            return None;
        }
        let (DexValue::String(language), DexValue::String(country), DexValue::String(variant)) =
            (language, country, variant)
        else {
            return match (language, country, variant) {
                (DexValue::Null, _, _) | (_, DexValue::Null, _) | (_, _, DexValue::Null) => {
                    Some(DexExecutionResult::Throw(self.allocate_object(
                        "Ljava/lang/NullPointerException;".to_owned(),
                    )))
                }
                _ => None,
            };
        };

        let parts = LocaleParts {
            language: normalized_language(language),
            script: String::new(),
            region: country.to_ascii_uppercase(),
            variant: variant.to_string(),
        };
        for (name, value) in [
            ("language", parts.language),
            ("region", parts.region),
            ("variant", parts.variant),
        ] {
            self.write_stored_instance_field(
                locale,
                &locale_field(name),
                DexValue::String(std::rc::Rc::from(value)),
            )
            .then_some(())?;
        }

        Some(DexExecutionResult::Void)
    }

    pub(in crate::dex_run) fn locale_parts_of(&self, locale: &DexValue) -> Option<LocaleParts> {
        match locale {
            DexValue::Reference(reference) => {
                locale_parts(reference.strip_prefix(LOCALE_REFERENCE_PREFIX)?)
            }
            DexValue::Object(_) => self.stored_locale_parts(locale),
            _ => None,
        }
    }

    fn stored_locale_parts(&self, locale: &DexValue) -> Option<LocaleParts> {
        let read =
            |name: &str| match self.read_stored_instance_field(locale, &locale_field(name))? {
                DexValue::String(value) => Some(value.to_string()),
                _ => None,
            };

        Some(LocaleParts {
            language: read("language")?,
            script: String::new(),
            region: read("region")?,
            variant: read("variant")?,
        })
    }
}

pub(super) fn locale_accessor_value(
    program: &DexProgram,
    accessor: LocaleAccessor,
    invocation: &DexInvocation,
) -> Option<DexValue> {
    let [receiver] = invocation.arguments.as_slice() else {
        return None;
    };
    let parts = program.locale_parts_of(receiver)?;
    let value = match accessor {
        // cite: libcore Locale.java#getLanguage
        LocaleAccessor::Language => parts.language,
        // cite: libcore Locale.java#getCountry
        LocaleAccessor::Country => parts.region,
        // cite: libcore Locale.java#getVariant
        LocaleAccessor::Variant => parts.variant,
    };

    Some(DexValue::String(std::rc::Rc::from(value)))
}

fn locale_parts(name: &str) -> Option<LocaleParts> {
    let constant = |language: &str, region: &str| LocaleParts {
        language: language.to_owned(),
        script: String::new(),
        region: region.to_owned(),
        variant: String::new(),
    };

    match name {
        // cite: libcore Locale.java#initDefault
        "default" => Some(constant("en", "US")),
        // cite: libcore Locale.java#ROOT, ENGLISH, US, UK, CANADA, CANADA_FRENCH, FRANCE, GERMAN
        "ROOT" => Some(constant("", "")),
        "ENGLISH" => Some(constant("en", "")),
        "US" => Some(constant("en", "US")),
        "UK" => Some(constant("en", "GB")),
        "CANADA" => Some(constant("en", "CA")),
        "CANADA_FRENCH" => Some(constant("fr", "CA")),
        "FRANCE" => Some(constant("fr", "FR")),
        "GERMAN" => Some(constant("de", "")),
        tag => language_tag_parts(tag),
    }
}

// cite: libcore LanguageTag.java#parse, parseLanguage, parseExtlangs, parseScript, parseRegion, parseVariants
// cite: libcore InternalLocaleBuilder.java#setLanguageTag
fn language_tag_parts(tag: &str) -> Option<LocaleParts> {
    if LEGACY_LANGUAGE_TAGS.contains(&tag.to_ascii_lowercase().as_str()) {
        return None;
    }

    let mut subtags = tag.split('-').peekable();
    let mut parts = LocaleParts {
        language: String::new(),
        script: String::new(),
        region: String::new(),
        variant: String::new(),
    };

    let Some(language) = subtags.next_if(|subtag| is_language(subtag)) else {
        return Some(parts);
    };
    let mut extlangs = Vec::new();
    while extlangs.len() < 3 {
        match subtags.next_if(|subtag| is_extlang(subtag)) {
            Some(extlang) => extlangs.push(extlang),
            None => break,
        }
    }
    let language = match extlangs.first() {
        Some(extlang) => extlang,
        None if language == "und" => "",
        None => language,
    };
    parts.language = normalized_language(language);

    if let Some(script) = subtags.next_if(|subtag| is_script(subtag)) {
        parts.script = title_case_script(script);
    }
    if let Some(region) = subtags.next_if(|subtag| is_region(subtag)) {
        parts.region = region.to_ascii_uppercase();
    }
    let mut variants = Vec::new();
    while let Some(variant) = subtags.next_if(|subtag| is_variant(subtag)) {
        variants.push(variant);
    }
    parts.variant = variants.join("_");

    Some(parts)
}

// cite: libcore LanguageTag.java#canonicalizeScript
fn title_case_script(script: &str) -> String {
    let (first, rest) = script.split_at(1);
    format!(
        "{}{}",
        first.to_ascii_uppercase(),
        rest.to_ascii_lowercase()
    )
}

// cite: libcore BaseLocale.java#getInstance
fn normalized_language(language: &str) -> String {
    match language.to_ascii_lowercase().as_str() {
        "he" => "iw".to_owned(),
        "yi" => "ji".to_owned(),
        "id" => "in".to_owned(),
        lowered => lowered.to_owned(),
    }
}

// cite: libcore LanguageTag.java#isLanguage
fn is_language(subtag: &str) -> bool {
    (2..=8).contains(&subtag.len()) && subtag.bytes().all(|byte| byte.is_ascii_alphabetic())
}

// cite: libcore LanguageTag.java#isExtlang
fn is_extlang(subtag: &str) -> bool {
    subtag.len() == 3 && subtag.bytes().all(|byte| byte.is_ascii_alphabetic())
}

// cite: libcore LanguageTag.java#isScript
fn is_script(subtag: &str) -> bool {
    subtag.len() == 4 && subtag.bytes().all(|byte| byte.is_ascii_alphabetic())
}

// cite: libcore LanguageTag.java#isRegion
fn is_region(subtag: &str) -> bool {
    let bytes = subtag.as_bytes();
    (bytes.len() == 2 && bytes.iter().all(u8::is_ascii_alphabetic))
        || (bytes.len() == 3 && bytes.iter().all(u8::is_ascii_digit))
}

// cite: libcore LanguageTag.java#isVariant
fn is_variant(subtag: &str) -> bool {
    let bytes = subtag.as_bytes();
    match bytes.len() {
        5..=8 => bytes.iter().all(u8::is_ascii_alphanumeric),
        4 => bytes[0].is_ascii_digit() && bytes[1..].iter().all(u8::is_ascii_alphanumeric),
        _ => false,
    }
}

// cite: libcore LanguageTag.java#parseLocale, isPrivateuseSubtag, PRIVUSE_VARIANT_PREFIX
// cite: libcore Locale.java#toLanguageTag
pub(in crate::dex_run) fn locale_language_tag(parts: &LocaleParts) -> String {
    let mut language = match parts.language.to_ascii_lowercase().as_str() {
        "iw" => "he".to_owned(),
        "ji" => "yi".to_owned(),
        "in" => "id".to_owned(),
        language if is_language(language) => language.to_owned(),
        _ => String::new(),
    };
    let script = is_script(&parts.script).then(|| canonical_script(&parts.script));
    let region = is_region(&parts.region).then(|| parts.region.to_ascii_uppercase());
    let mut variant = parts.variant.as_str();
    if language == "no" && region.as_deref() == Some("NO") && variant == "NY" {
        language = "nn".to_owned();
        variant = "";
    }
    let mut subtags = variant
        .split('_')
        .filter(|subtag| !subtag.is_empty())
        .peekable();
    let mut variants = Vec::new();
    while let Some(subtag) = subtags.next_if(|subtag| is_variant(subtag)) {
        variants.push(subtag);
    }
    let private_use: Vec<&str> = subtags
        .take_while(|subtag| {
            (1..=8).contains(&subtag.len())
                && subtag.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
        .collect();
    let has_subtag = script.is_some() || region.is_some() || !variants.is_empty();
    if language.is_empty() && (has_subtag || private_use.is_empty()) {
        language = "und".to_owned();
    }

    let mut tag: Vec<String> = [Some(language), script, region]
        .into_iter()
        .flatten()
        .filter(|subtag| !subtag.is_empty())
        .collect();
    tag.extend(variants.into_iter().map(str::to_owned));
    if !private_use.is_empty() {
        tag.push(format!("x-lvariant-{}", private_use.join("-")));
    }

    tag.join("-")
}

fn canonical_script(script: &str) -> String {
    let lower = script.to_ascii_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
        .unwrap_or_default()
}

#[cfg(test)]
pub(in crate::dex_run) fn java_locale_parts(
    language: &str,
    region: &str,
    variant: &str,
) -> LocaleParts {
    LocaleParts {
        language: language.to_owned(),
        script: String::new(),
        region: region.to_owned(),
        variant: variant.to_owned(),
    }
}
