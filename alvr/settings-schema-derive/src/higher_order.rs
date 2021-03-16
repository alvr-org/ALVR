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

        gui: ChoiceControlType,
    },
    Boolean {
        default: bool,
    },
    Action,
}

#[derive(FromMeta)]
enum UpdateType {
    Assign,
    Remove,
}

#[derive(FromMeta)]
struct ModifierDesc {
    target: String,
    update_op: UpdateType,
    expr: String,

    #[darling(multiple)]
    #[darling(rename = "var")]
    vars: Vec<String>,
}

#[derive(FromMeta)]
pub struct HigherOrderSetting {
    name: String,

    #[darling(rename = "data")]
    data_type: HigherOrderType,

    #[darling(multiple)]
    #[darling(rename = "modifier")]
    modifiers: Vec<ModifierDesc>,
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
                ChoiceControlType::Dropdown => quote!(settings_schema::ChoiceControlType::DropDown),
                ChoiceControlType::ButtonGroup => {
                    quote!(settings_schema::ChoiceControlType::ButtonGroup)
                }
            };

            quote!(settings_schema::HigherOrderType::Choice {
                default: #default.into()
                variants: vec![#(#variants.into()),*],
                gui: #gui_ts,
            })
        }
        HigherOrderType::Boolean { default } => {
            quote!(settings_schema::HigherOrderType::Boolean { default: #default })
        }
        HigherOrderType::Action => quote!(settings_schema::HigherOrderType::Action),
    };

    let mut modifiers_ts = vec![];
    for m in &setting.modifiers {
        let target_path_ts = &m.target;
        let update_type_ts = match m.update_op {
            UpdateType::Assign => quote!(settings_schema::UpdateType::Assign),
            UpdateType::Remove => quote!(settings_schema::UpdateType::Remove),
        };
        let expr = &m.expr;
        let modifier_vars_ts = &m.vars;

        modifiers_ts.push(quote!(settings_schema::ModifierDesc {
            target: #target_path_ts.to_owned(),
            update_operation: #update_type_ts,
            expression: #expr.into(),
            variables: vec![#(#modifier_vars_ts.to_owned()),*]
        }))
    }

    Ok(Entry {
        key: key.clone(),
        entry_type_ts: quote!(settings_schema::EntryType::HigherOrder {
            data_type: #data_type_ts,
            modifiers: vec![#(#modifiers_ts),*],
        }),
    })
}
