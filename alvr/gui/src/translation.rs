// use alvr_common::prelude::*;
// use fluent::{FluentArgs, FluentBundle, FluentResource};
// use fluent_langneg::NegotiationStrategy;
// use fluent_syntax::ast::Pattern;
// use std::{borrow::ToOwned, rc::Rc};
// use unic_langid::LanguageIdentifier;
// use yew::{html, Children, ContextProvider, Properties};
// use yew_functional::{function_component, use_context};

// async fn get_bundle(locale: &LanguageIdentifier) -> StrResult<FluentBundle<FluentResource>> {
//     let resource_future = {
//         let locale = locale.clone();
//         async move {
//             trace_err!(
//                 trace_err!(
//                     reqwest::get(format!(
//                         "{}/languages/{}.ftl",
//                         crate::get_base_url(),
//                         locale
//                     ))
//                     .await
//                 )?
//                 .text()
//                 .await
//             )
//         }
//     };

//     let resource = match resource_future.await {
//         Ok(resource) => resource,
//         Err(e) => return fmt_e!("Failed to load \"{}\" language resource: {}", locale, e),
//     };

//     let resource = match FluentResource::try_new(resource) {
//         Ok(resource) => resource,
//         Err((_, errors)) => return fmt_e!("Failed to parse language resource: {:#?}", errors),
//     };

//     let mut bundle = FluentBundle::new(vec![locale.clone()]);
//     bundle.add_resource_overriding(resource);

//     Ok(bundle)
// }

// fn format_pattern(
//     bundle: &FluentBundle<FluentResource>,
//     pattern: &Pattern<&str>,
//     args: Option<&FluentArgs>,
// ) -> String {
//     let mut errors = vec![];
//     let value = bundle.format_pattern(pattern, args, &mut errors);
//     if errors.is_empty() {
//         value.as_ref().to_owned()
//     } else {
//         error!(
//             "Translation error: pattern: \"{:?}\", errors: {:#?}",
//             pattern, errors
//         );
//         "ERROR: see log".to_string()
//     }
// }

// fn get_message_value(
//     bundle: &FluentBundle<FluentResource>,
//     message_id: &str,
//     args: Option<&FluentArgs>,
// ) -> Option<String> {
//     bundle
//         .get_message(message_id)
//         .map(|message| {
//             message
//                 .value()
//                 .map(|pattern| format_pattern(bundle, pattern, args))
//         })
//         .flatten()
// }

// fn get_attribute(
//     bundle: &FluentBundle<FluentResource>,
//     message_id: &str,
//     attribute_id: &str,
//     args: Option<&FluentArgs>,
// ) -> Option<String> {
//     bundle
//         .get_message(message_id)
//         .map(|message| {
//             message
//                 .get_attribute(attribute_id)
//                 .map(|attribute| format_pattern(bundle, attribute.value(), args))
//         })
//         .flatten()
// }

// pub struct TranslationManager {
//     preferred_bundle: FluentBundle<FluentResource>,
//     fallback_bundle: FluentBundle<FluentResource>,
// }

// impl TranslationManager {
//     pub async fn new(code: Option<String>) -> StrResult<Self> {
//         let code = if let Some(code) = code {
//             code
//         } else {
//             trace_none!(trace_none!(web_sys::window())?.navigator().language())?
//         };
//         let requested_language = trace_err!(code.parse::<LanguageIdentifier>())?;

//         let languages_list_json = trace_err!(
//             trace_err!(
//                 reqwest::get(format!("{}/languages/list.json", crate::get_base_url())).await
//             )?
//             .json::<serde_json::Value>()
//             .await
//         )?;

//         let mut available_languages = vec![];
//         for key in trace_none!(languages_list_json.as_object())?.keys() {
//             let lang_code = trace_err!(key.parse())?;
//             available_languages.push(lang_code);
//         }

//         let fallback_language = trace_err!("en".parse::<LanguageIdentifier>())?;

//         let resolved_locales = fluent_langneg::negotiate_languages(
//             &[requested_language],
//             &available_languages,
//             Some(&fallback_language),
//             NegotiationStrategy::Filtering,
//         );

//         let preferred_bundle = get_bundle(trace_none!(resolved_locales.first())?).await?;
//         let fallback_bundle = get_bundle(trace_none!(resolved_locales.last())?).await?;

//         Ok(TranslationManager {
//             preferred_bundle,
//             fallback_bundle,
//         })
//     }

//     pub fn fallible_with_args(&self, key: &str, args: Option<&FluentArgs>) -> Option<String> {
//         get_message_value(&self.preferred_bundle, key, args)
//             .or_else(|| get_message_value(&self.fallback_bundle, key, args))
//     }

//     pub fn with_args(&self, key: &str, args: FluentArgs) -> String {
//         self.fallible_with_args(key, Some(&args))
//             .unwrap_or_else(|| key.to_owned())
//     }

//     pub fn get(&self, key: &str) -> String {
//         self.fallible_with_args(key, None)
//             .unwrap_or_else(|| key.to_owned())
//     }

//     pub fn attribute_fallible_with_args(
//         &self,
//         key: &str,
//         attribute_key: &str,
//         args: Option<&FluentArgs>,
//     ) -> Option<String> {
//         get_attribute(&self.preferred_bundle, key, attribute_key, args)
//             .or_else(|| get_attribute(&self.fallback_bundle, key, attribute_key, args))
//     }

//     pub fn attribute_with_args(&self, key: &str, attribute_key: &str, args: FluentArgs) -> String {
//         self.attribute_fallible_with_args(key, attribute_key, Some(&args))
//             .unwrap_or_else(|| attribute_key.to_owned())
//     }

//     pub fn attribute(&self, key: &str, attribute_key: &str) -> String {
//         self.attribute_fallible_with_args(key, attribute_key, None)
//             .unwrap_or_else(|| attribute_key.to_owned())
//     }
// }

// #[derive(Clone)]
// pub struct TransContext {
//     pub manager: Rc<TranslationManager>,
// }

// // PartialEq must be implemented manually because Localization does not implement it
// impl PartialEq for TransContext {
//     fn eq(&self, other: &Self) -> bool {
//         Rc::ptr_eq(&self.manager, &other.manager)
//     }
// }

// #[derive(Properties, Clone, PartialEq)]
// pub struct TransProviderProps {
//     pub context: TransContext,
//     pub children: Children,
// }

// #[function_component(TransProvider)]
// pub fn trans_provider(props: &TransProviderProps) -> Html {
//     html! {
//         <ContextProvider<TransContext> context=props.context.clone()>
//             {props.children.clone()}
//         </ContextProvider<TransContext>>
//     }
// }

// pub fn use_translation() -> Rc<TranslationManager> {
//     Rc::clone(&use_context::<TransContext>().unwrap().manager)
// }

// #[derive(Clone, PartialEq)]
// struct SettingsTransContext(Vec<String>);

// #[derive(Properties, Clone, PartialEq)]
// pub struct SettingsTransPathProviderProps {
//     pub children: Children,
// }

// #[function_component(SettingsTransPathProvider)]
// pub fn settings_trans_path_provider(props: &SettingsTransPathProviderProps) -> Html {
//     html! {
//         <ContextProvider<SettingsTransContext> context=SettingsTransContext(vec![])>
//             {props.children.clone()}
//         </ContextProvider<SettingsTransContext>>
//     }
// }

// #[derive(Properties, Clone, PartialEq)]
// pub struct SettingsTransNodeProps {
//     pub subkey: String,
//     pub children: Children,
// }

// #[function_component(SettingsTransNode)]
// pub fn settings_trans_node(props: &SettingsTransNodeProps) -> Html {
//     let mut context = use_context::<SettingsTransContext>().unwrap().0;
//     context.push(props.subkey.clone());

//     html! {
//         <ContextProvider<Vec<String>> context=context>
//             {props.children.clone()}
//         </ContextProvider<Vec<String>>>
//     }
// }

// pub fn use_setting_name_trans(subkey: &str) -> String {
//     let manager = use_translation();

//     let mut route_segments = use_context::<SettingsTransContext>().unwrap().0;
//     route_segments.push(subkey.to_owned());

//     let route = route_segments.join("-");

//     if let Some(name) = manager.fallible_with_args(&route, None) {
//         name
//     } else {
//         subkey.into()
//     }
// }

// pub struct SettingsTrans {
//     pub name: String,
//     pub help: Option<String>,
//     pub notice: Option<String>,
// }

// pub fn use_setting_trans(subkey: &str) -> SettingsTrans {
//     let manager = use_translation();

//     let mut route_segments = use_context::<Vec<String>>().expect("Trans context");
//     route_segments.push(subkey.to_owned());

//     let route = route_segments.join("-");

//     if let Some(name) = manager.fallible_with_args(&route, None) {
//         SettingsTrans {
//             name,
//             help: manager.attribute_fallible_with_args(&route, "help", None),
//             notice: manager.attribute_fallible_with_args(&route, "notice", None),
//         }
//     } else {
//         SettingsTrans {
//             name: subkey.to_owned(),
//             help: None,
//             notice: None,
//         }
//     }
// }
