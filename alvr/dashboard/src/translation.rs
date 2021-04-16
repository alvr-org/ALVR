use alvr_common::prelude::*;
use fluent::{FluentArgs, FluentBundle, FluentResource};
use fluent_fallback::{
    generator::{BundleGenerator, FluentBundleResult},
    types::{L10nKey, L10nMessage},
    Localization,
};
use fluent_langneg::NegotiationStrategy;
use futures::Stream;
use std::{
    borrow::{Cow, ToOwned},
    cell::RefCell,
    collections::VecDeque,
    pin::Pin,
    rc::Rc,
    task::{self, Poll},
};
use unic_langid::LanguageIdentifier;
use yew::{html, Children, Properties};
use yew_functional::{function_component, use_context, ContextProvider};

struct TranslationSourceStream {
    prefetched_bundle_results: VecDeque<FluentBundleResult<FluentResource>>,
}

impl Iterator for TranslationSourceStream {
    type Item = FluentBundleResult<FluentResource>;

    fn next(&mut self) -> Option<Self::Item> {
        self.prefetched_bundle_results.pop_front()
    }
}

// Unused, required by fluent
impl Stream for TranslationSourceStream {
    type Item = FluentBundleResult<FluentResource>;

    fn poll_next(self: Pin<&mut Self>, _: &mut task::Context<'_>) -> Poll<Option<Self::Item>> {
        unreachable!()
    }
}

struct TranslationSources {
    prefetched_bundle_results: RefCell<Vec<FluentBundleResult<FluentResource>>>,
}

impl BundleGenerator for TranslationSources {
    type Resource = FluentResource;
    type Iter = TranslationSourceStream;
    type Stream = TranslationSourceStream;

    fn bundles_iter(
        &self,
        _: <Vec<LanguageIdentifier> as IntoIterator>::IntoIter,
        _: Vec<String>,
    ) -> TranslationSourceStream {
        TranslationSourceStream {
            prefetched_bundle_results: self
                .prefetched_bundle_results
                .borrow_mut()
                .drain(..)
                .collect(),
        }
    }
}

pub struct TranslationManager {
    localization: Localization<TranslationSources, Vec<LanguageIdentifier>>,
}

impl TranslationManager {
    pub async fn new(code: Option<String>) -> StrResult<Self> {
        let code = if let Some(code) = code {
            code
        } else {
            trace_none!(trace_none!(web_sys::window())?.navigator().language())?
        };
        let requested_language = trace_err!(code.parse::<LanguageIdentifier>())?;

        let languages_list_json = trace_err!(
            trace_err!(
                reqwest::get(format!("{}/languages/list.json", crate::get_base_url())).await
            )?
            .json::<serde_json::Value>()
            .await
        )?;

        let mut available_languages = vec![];
        for key in trace_none!(languages_list_json.as_object())?.keys() {
            let lang_code = trace_err!(key.parse())?;
            available_languages.push(lang_code);
        }

        let default_code = trace_err!("en".parse())?;

        let resolved_locales: Vec<&LanguageIdentifier> = fluent_langneg::negotiate_languages(
            &[requested_language],
            &available_languages,
            Some(&default_code),
            NegotiationStrategy::Filtering,
        );

        let mut bundle_results = vec![];

        for locale in resolved_locales.iter().cloned() {
            let resource_future = {
                let locale = locale.clone();
                async move {
                    trace_err!(
                        trace_err!(
                            reqwest::get(format!(
                                "{}/languages/{}.ftl",
                                crate::get_base_url(),
                                locale
                            ))
                            .await
                        )?
                        .text()
                        .await
                    )
                }
            };

            let resource = match resource_future.await {
                Ok(resource) => resource,
                Err(e) => {
                    error!("Failed to load \"{}\" language resource: {}", locale, e);
                    continue;
                }
            };

            let mut bundle = FluentBundle::new(vec![locale.clone()]);

            let resource = match FluentResource::try_new(resource) {
                Ok(resource) => resource,
                Err((_, errors)) => {
                    bundle_results
                        .push(Err((bundle, errors.into_iter().map(Into::into).collect())));
                    continue;
                }
            };

            if let Err(errors) = bundle.add_resource(resource) {
                bundle_results.push(Err((bundle, errors.into_iter().map(Into::into).collect())));
                continue;
            }

            bundle_results.push(Ok(bundle));
        }

        let localization = Localization::with_env(
            vec![],
            true,
            vec![],
            TranslationSources {
                prefetched_bundle_results: RefCell::new(bundle_results),
            },
        );

        Ok(Self { localization })
    }

    pub fn get<'a>(&'a self, key: &'a str) -> Cow<'a, str> {
        match self.localization.format_value_sync(key, None, &mut vec![]) {
            Ok(Some(value)) => value,
            Ok(None) => key.to_owned().into(),
            Err(e) => {
                error!("{}", e);
                key.to_owned().into()
            }
        }
    }

    pub fn get_with_args<'a>(&'a self, key: &'a str, args: &'a FluentArgs) -> Cow<'a, str> {
        match self
            .localization
            .format_value_sync(key, Some(&args), &mut vec![])
        {
            Ok(Some(value)) => value,
            Ok(None) => key.to_owned().into(),
            Err(e) => {
                error!("{}", e);
                key.to_owned().into()
            }
        }
    }

    pub fn get_with_attributes<'a>(&'a self, keys: &'a [L10nKey<'a>]) -> Vec<Option<L10nMessage>> {
        match self.localization.format_messages_sync(keys, &mut vec![]) {
            Ok(messages) => messages,
            Err(e) => {
                error!("{}", e);
                vec![]
            }
        }
    }
}

#[derive(Clone)]
pub struct TransContext {
    pub manager: Rc<TranslationManager>,
}

// PartialEq must be implemented manually because Localization does not implement it
impl PartialEq for TransContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.manager, &other.manager)
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct TransProviderProps {
    pub context: TransContext,
    pub children: Children,
}

#[function_component(TransProvider)]
pub fn trans_provider(props: &TransProviderProps) -> Html {
    html! {
        <ContextProvider<TransContext> context=props.context.clone()>
            {props.children.clone()}
        </ContextProvider<TransContext>>
    }
}

pub fn use_translation() -> Rc<TranslationManager> {
    use_context::<TransContext>()
        .expect("Trans context")
        .manager
        .clone()
}

pub fn use_trans(key: &str) -> String {
    let manager = use_translation();
    manager.get(key).as_ref().to_owned()
}

#[derive(Properties, Clone, PartialEq)]
pub struct TransNameProps {
    #[prop_or_default]
    key: String,
}

#[function_component(Trans)]
pub fn trans(props: &TransNameProps) -> Html {
    html!({ use_trans(&props.key) })
}

#[derive(Properties, Clone, PartialEq)]
pub struct SettingsTransPathProviderProps {
    pub children: Children,
}

#[function_component(SettingsTransPathProvider)]
pub fn settings_trans_path_provider(props: &TransProviderProps) -> Html {
    html! {
        <ContextProvider<Vec<String>> context=vec![]>
            {props.children.clone()}
        </ContextProvider<Vec<String>>>
    }
}

pub struct SettingsTrans {
    name: String,
    help: Option<String>,
    notice: Option<String>,
}

pub fn use_settings_trans(subkey: &str) -> SettingsTrans {
    let manager = use_translation();

    let mut route_segments = (*use_context::<Vec<String>>().expect("Trans context")).clone();
    route_segments.push(subkey.to_owned());

    let route = route_segments.join(".");

    let keys = vec![L10nKey {
        id: route.into(),
        args: None,
    }];

    let message = manager.get_with_attributes(&keys).pop();

    if let Some(Some(message)) = message {
        let name = message.value.as_deref().map(ToOwned::to_owned);

        let mut help = None;
        let mut notice = None;
        for attribute in message.attributes {
            if attribute.name == "help" {
                help = Some(attribute.value.as_ref().to_owned())
            } else if attribute.name == "notice" {
                notice = Some(attribute.value.as_ref().to_owned())
            } else {
                error!("Unexpected translation attribute: {}", attribute.name)
            }
        }

        SettingsTrans {
            name: name.unwrap_or_else(|| subkey.to_owned()),
            help,
            notice,
        }
    } else {
        SettingsTrans {
            name: subkey.to_owned(),
            help: None,
            notice: None,
        }
    }
}
