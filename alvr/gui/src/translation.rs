use alvr_common::prelude::*;
use fluent::{FluentArgs, FluentBundle, FluentResource};
use fluent_langneg::NegotiationStrategy;
use fluent_syntax::ast::Pattern;
use std::{borrow::ToOwned, collections::BTreeMap, sync::Arc};
use unic_langid::LanguageIdentifier;

fn get_bundle(locale: &LanguageIdentifier, raw: String) -> StrResult<FluentBundle<FluentResource>> {
    let resource = match FluentResource::try_new(raw) {
        Ok(resource) => resource,
        Err((_, errors)) => return fmt_e!("Failed to parse language resource: {:#?}", errors),
    };

    let mut bundle = FluentBundle::new(vec![locale.clone()]);
    bundle.add_resource_overriding(resource);

    Ok(bundle)
}

fn format_pattern(
    bundle: &FluentBundle<FluentResource>,
    pattern: &Pattern<&str>,
    args: Option<&FluentArgs>,
) -> String {
    let mut errors = vec![];
    let value = bundle.format_pattern(pattern, args, &mut errors);
    if errors.is_empty() {
        value.as_ref().to_owned()
    } else {
        error!(
            "Translation error: pattern: \"{:?}\", errors: {:#?}",
            pattern, errors
        );
        "ERROR: see log".to_string()
    }
}

fn get_message_value(
    bundle: &FluentBundle<FluentResource>,
    message_id: &str,
    args: Option<&FluentArgs>,
) -> Option<String> {
    bundle
        .get_message(message_id)
        .map(|message| {
            message
                .value()
                .map(|pattern| format_pattern(bundle, pattern, args))
        })
        .flatten()
}

fn get_attribute(
    bundle: &FluentBundle<FluentResource>,
    message_id: &str,
    attribute_id: &str,
    args: Option<&FluentArgs>,
) -> Option<String> {
    bundle
        .get_message(message_id)
        .map(|message| {
            message
                .get_attribute(attribute_id)
                .map(|attribute| format_pattern(bundle, attribute.value(), args))
        })
        .flatten()
}

pub struct TranslationBundle {
    languages: BTreeMap<String, String>,
    preferred_bundle: FluentBundle<FluentResource>,
    fallback_bundle: FluentBundle<FluentResource>,
}

impl TranslationBundle {
    pub fn new(
        code: Option<String>,
        list_raw: &str,
        get_raw_bundle: impl Fn(&LanguageIdentifier) -> String,
    ) -> StrResult<Self> {
        let code = if let Some(code) = code {
            code
        } else {
            #[cfg(not(target_os = "android"))]
            let code = locale_config::Locale::user_default().to_string();
            #[cfg(target_os = "android")]
            let code = "en".to_owned();

            code
        };
        let requested_language = trace_err!(code.parse::<LanguageIdentifier>())?;

        let languages = trace_err!(serde_json::from_str::<BTreeMap<String, String>>(list_raw))?;

        let mut available_languages = vec![];
        for key in languages.keys() {
            let lang_code = trace_err!(key.parse())?;
            available_languages.push(lang_code);
        }

        let fallback_language = "en".parse::<LanguageIdentifier>().unwrap();

        let resolved_locales = fluent_langneg::negotiate_languages(
            &[requested_language],
            &available_languages,
            Some(&fallback_language),
            NegotiationStrategy::Filtering,
        );

        let prederred_locale = trace_none!(resolved_locales.first())?;
        let fallback_locale = trace_none!(resolved_locales.last())?;

        let preferred_bundle = get_bundle(prederred_locale, get_raw_bundle(prederred_locale))?;
        let fallback_bundle = get_bundle(fallback_locale, get_raw_bundle(fallback_locale))?;

        Ok(TranslationBundle {
            languages,
            preferred_bundle,
            fallback_bundle,
        })
    }

    pub fn fallible_with_args(&self, key: &str, args: Option<&FluentArgs>) -> Option<String> {
        get_message_value(&self.preferred_bundle, key, args)
            .or_else(|| get_message_value(&self.fallback_bundle, key, args))
    }

    pub fn with_args(&self, key: &str, args: FluentArgs) -> String {
        self.fallible_with_args(key, Some(&args))
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn get(&self, key: &str) -> String {
        self.fallible_with_args(key, None)
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn attribute_fallible_with_args(
        &self,
        key: &str,
        attribute_key: &str,
        args: Option<&FluentArgs>,
    ) -> Option<String> {
        get_attribute(&self.preferred_bundle, key, attribute_key, args)
            .or_else(|| get_attribute(&self.fallback_bundle, key, attribute_key, args))
    }

    pub fn attribute_with_args(&self, key: &str, attribute_key: &str, args: FluentArgs) -> String {
        self.attribute_fallible_with_args(key, attribute_key, Some(&args))
            .unwrap_or_else(|| attribute_key.to_owned())
    }

    pub fn attribute(&self, key: &str, attribute_key: &str) -> String {
        self.attribute_fallible_with_args(key, attribute_key, None)
            .unwrap_or_else(|| attribute_key.to_owned())
    }

    pub fn languages(&self) -> &BTreeMap<String, String> {
        &self.languages
    }
}

pub struct SharedTranslation {
    pub ok: String,
    pub cancel: String,
    pub do_not_ask_again: String,
    pub reset_to: String,
}

pub fn get_shared_translation(manager: &TranslationBundle) -> Arc<SharedTranslation> {
    Arc::new(SharedTranslation {
        ok: manager.get("ok"),
        cancel: manager.get("cancel"),
        do_not_ask_again: manager.get("do-not-ask-again"),
        reset_to: manager.get("reset-to"),
    })
}
