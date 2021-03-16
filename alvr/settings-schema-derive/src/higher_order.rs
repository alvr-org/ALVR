//! Higher order settings live only inside the schema and represent instructions to modify other
// contrete settings or HOS.

use crate::{ChoiceControlType, TResult, TokenStream2};
use darling::FromMeta;
use quote::quote;

#[derive(FromMeta)]
enum HigherOrderType {
    Choice {
        default: String,

        #[darling(multiple)]
        #[darling(rename = "variant")]
        variants: Vec<String>,

        #[darling(default)]
        gui: Option<ChoiceControlType>,
    },
    Boolean {
        default: bool,
    },
    Action,
}

#[derive(FromMeta)]
pub struct HigherOrderSetting {
    name: String,

    #[darling(rename = "data")]
    data_type: HigherOrderType,

    #[darling(multiple)]
    #[darling(rename = "modifier")]
    modifiers: Vec<String>,
}

pub struct Entry {
    // Name of the higher order setting
    pub key: String,

    // schema instantiation code for the current higher order setting
    pub entry_type_ts: TokenStream2,
}

pub fn schema(setting: &HigherOrderSetting) -> TResult<Entry> {
    let key = &setting.name;

    let data_type_ts = match &setting.data_type {
        HigherOrderType::Choice {
            default,
            variants,
            gui,
        } => {
            let gui_ts = match gui {
                None => quote!(None),
                Some(ChoiceControlType::Dropdown) => {
                    quote!(Some(settings_schema::ChoiceControlType::Dropdown))
                }
                Some(ChoiceControlType::ButtonGroup) => {
                    quote!(Some(settings_schema::ChoiceControlType::ButtonGroup))
                }
            };

            quote!(settings_schema::HigherOrderType::Choice {
                default: #default.into(),
                variants: vec![#(#variants.into()),*],
                gui: #gui_ts,
            })
        }
        HigherOrderType::Boolean { default } => {
            quote!(settings_schema::HigherOrderType::Boolean { default: #default })
        }
        HigherOrderType::Action => quote!(settings_schema::HigherOrderType::Action),
    };

    let modifiers_ts = &setting.modifiers;

    Ok(Entry {
        key: key.clone(),
        entry_type_ts: quote!(settings_schema::EntryType::HigherOrder {
            data_type: #data_type_ts,
            modifiers: vec![#(#modifiers_ts.into()),*],
        }),
    })
}
