use crate::{error, suffix_ident, FieldMeta, TResult, TokenStream2};
use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{quote, ToTokens};
use syn::{GenericArgument, Lit, PathArguments, Type, TypeArray, TypePath};

#[derive(FromMeta)]
pub enum NumericGuiType {
    TextBox,
    UpDown,
    Slider,
}

pub struct SchemaData {
    // Schema representation type, assigned to a specific field in the schema representation struct
    pub default_ty_ts: TokenStream2,

    // Schema instatiation code for a specific field
    pub schema_code_ts: TokenStream2,
}

fn get_only_type_argument(arguments: &PathArguments) -> &Type {
    if let PathArguments::AngleBracketed(args_block) = &arguments {
        if let GenericArgument::Type(ty) = args_block.args.first().unwrap() {
            return ty;
        }
    }
    // Fail cases are already handled by the compiler
    unreachable!()
}

fn forbid_numeric_attrs(field: &FieldMeta, type_str: &str) -> TResult<()> {
    let maybe_invalid_arg = field
        .min
        .as_ref()
        .or_else(|| field.max.as_ref())
        .or_else(|| field.step.as_ref());

    let tokens = if let Some(arg) = maybe_invalid_arg {
        arg.to_token_stream()
    } else if field.gui.is_some() {
        quote!()
    } else {
        return Ok(());
    };

    error(
        &format!("Unexpected argument for {} type", type_str),
        tokens,
    )
}

fn bool_type_schema(field: &FieldMeta) -> TResult {
    forbid_numeric_attrs(field, "bool")?;

    Ok(quote!(SchemaNode::Boolean(default)))
}

fn maybe_integer_literal(literal: Option<&Lit>) -> TResult {
    if let Some(literal) = literal {
        if let Lit::Int(lit_int) = literal {
            Ok(quote!(Some(#lit_int)))
        } else {
            error("Expected integer literal", literal)
        }
    } else {
        Ok(quote!(None))
    }
}

fn maybe_float_literal(literal: Option<&Lit>) -> TResult {
    if let Some(literal) = literal {
        if let Lit::Float(lit_float) = literal {
            Ok(quote!(Some(#lit_float as _)))
        } else {
            error("Expected float literal", literal)
        }
    } else {
        Ok(quote!(None))
    }
}

fn maybe_numeric_gui(gui: Option<&NumericGuiType>) -> proc_macro2::TokenStream {
    if let Some(gui) = gui {
        match gui {
            NumericGuiType::TextBox => quote!(Some(NumericGuiType::TextBox)),
            NumericGuiType::UpDown => quote!(Some(NumericGuiType::UpDown)),
            NumericGuiType::Slider => quote!(Some(NumericGuiType::Slider)),
        }
    } else {
        quote!(None)
    }
}

fn integer_type_schema(field: &FieldMeta) -> TResult {
    let min_ts = maybe_integer_literal(field.min.as_ref())?;
    let max_ts = maybe_integer_literal(field.max.as_ref())?;
    let step_ts = maybe_integer_literal(field.step.as_ref())?;
    let gui_ts = maybe_numeric_gui(field.gui.as_ref());

    Ok(quote!(SchemaNode::Integer(SchemaNumeric {
        default: default as _,
        min: #min_ts,
        max: #max_ts,
        step: #step_ts,
        gui: #gui_ts,
    })))
}

fn float_type_schema(field: &FieldMeta) -> TResult {
    let min_ts = maybe_float_literal(field.min.as_ref())?;
    let max_ts = maybe_float_literal(field.max.as_ref())?;
    let step_ts = maybe_float_literal(field.step.as_ref())?;
    let gui_ts = maybe_numeric_gui(field.gui.as_ref());

    Ok(quote!(SchemaNode::Float(SchemaNumeric {
        default: default as _,
        min: #min_ts,
        max: #max_ts,
        step: #step_ts,
        gui: #gui_ts,
    })))
}

fn string_type_schema(field: &FieldMeta) -> TResult {
    forbid_numeric_attrs(field, "String")?;

    Ok(quote!(SchemaNode::Text(default)))
}

fn custom_leaf_type_schema(ty_ident: &Ident, field: &FieldMeta) -> TResult {
    forbid_numeric_attrs(field, "custom")?;

    Ok(quote!(#ty_ident::schema(default)))
}

// Generate a default representation type and corresponding schema instantiation code.
// This function calls itself recursively to parse the whole compound type. The recursion degree is
// only 1: only types that have only one type argument can be parsed. Still custom types cannot have
// type arguments, so they are always the leaf type.
// The meta parameter contains the attributes associated to the curent field: they are forwarded
// as-is in every recursion step. Most of the attributes are used for numerical leaf types, but
// there is also the `switch_default` flag that is used by each Switch type inside the type chain.
pub(crate) fn schema(ty: &Type, meta: &FieldMeta) -> Result<SchemaData, TokenStream> {
    match &ty {
        Type::Array(TypeArray { len, elem, .. }) => {
            let SchemaData {
                default_ty_ts,
                schema_code_ts,
            } = schema(elem, meta)?;
            Ok(SchemaData {
                default_ty_ts: quote!([#default_ty_ts; #len]),
                schema_code_ts: quote! {{
                    let length = #len;
                    let content = std::array::IntoIter::new(default).map(|default| {
                        #schema_code_ts
                    }).collect::<Vec<_>>();

                    SchemaNode::Array(content)
                }},
            })
        }
        Type::Path(TypePath { path, .. }) => {
            let ty_last = path.segments.last().unwrap();
            let ty_ident = &ty_last.ident;
            if matches!(ty_last.arguments, PathArguments::None) {
                let mut custom_default_ty_ts = None;
                let schema_code_ts = match ty_ident.to_string().as_str() {
                    "bool" => bool_type_schema(meta)?,
                    "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "i128"
                    | "u128" | "isize" | "usize" => integer_type_schema(meta)?,
                    "f32" | "f64" => float_type_schema(meta)?,
                    "String" => string_type_schema(meta)?,
                    _ => {
                        custom_default_ty_ts =
                            Some(suffix_ident(&ty_ident, "Default").to_token_stream());
                        custom_leaf_type_schema(ty_ident, meta)?
                    }
                };
                Ok(SchemaData {
                    default_ty_ts: if let Some(tokens) = custom_default_ty_ts {
                        tokens
                    } else {
                        ty_ident.to_token_stream()
                    },
                    schema_code_ts,
                })
            } else if ty_ident == "Option" {
                let SchemaData {
                    default_ty_ts,
                    schema_code_ts,
                } = schema(get_only_type_argument(&ty_last.arguments), meta)?;
                Ok(SchemaData {
                    default_ty_ts: quote!(settings_schema::OptionalDefault<#default_ty_ts>),
                    schema_code_ts: quote! {{
                        let default_set = default.set;
                        let default = default.content;
                        let content = Box::new(#schema_code_ts);
                        SchemaNode::Optional(SchemaOptional { default_set, content })
                    }},
                })
            } else if ty_ident == "Switch" {
                let content_advanced = meta.switch_advanced.is_some();
                let SchemaData {
                    default_ty_ts,
                    schema_code_ts,
                } = schema(get_only_type_argument(&ty_last.arguments), meta)?;
                Ok(SchemaData {
                    default_ty_ts: quote!(settings_schema::SwitchDefault<#default_ty_ts>),
                    schema_code_ts: quote! {{
                        let default_enabled = default.enabled;
                        let default = default.content;
                        let content = Box::new(#schema_code_ts);
                        SchemaNode::Switch(SchemaSwitch {
                            default_enabled,
                            content_advanced: #content_advanced,
                            content
                        })
                    }},
                })
            } else if ty_ident == "Vec" {
                let ty_arg = get_only_type_argument(&ty_last.arguments);
                if let Type::Tuple(ty_tuple) = ty_arg {
                    if ty_tuple.elems.len() != 2 {
                        error("Expected two arguments", &ty_tuple.elems)
                    } else if ty_tuple.elems[0].to_token_stream().to_string() != "String" {
                        error("First argument must be a `String`", &ty_tuple.elems)
                    } else {
                        let ty_arg = &ty_tuple.elems[1];
                        let SchemaData {
                            default_ty_ts,
                            schema_code_ts,
                        } = schema(ty_arg, meta)?;
                        Ok(SchemaData {
                            default_ty_ts: quote! {
                                settings_schema::DictionaryDefault<#default_ty_ts>
                            },
                            schema_code_ts: quote! {{
                                let default_content =
                                    serde_json::to_value(default.content).unwrap();
                                let default_key = default.key;
                                let default = default.value;
                                let default_value = Box::new(#schema_code_ts);
                                SchemaNode::Dictionary(SchemaDictionary {
                                    default_key,
                                    default_value,
                                    default: default_content
                                })
                            }},
                        })
                    }
                } else {
                    let SchemaData {
                        default_ty_ts,
                        schema_code_ts,
                    } = schema(ty_arg, meta)?;
                    Ok(SchemaData {
                        default_ty_ts: quote!(settings_schema::VectorDefault<#default_ty_ts>),
                        schema_code_ts: quote! {{
                            let default_content =
                                serde_json::to_value(default.content).unwrap();
                            let default = default.element;
                            let default_element = Box::new(#schema_code_ts);
                            SchemaNode::Vector(SchemaVector {
                                default_element,
                                default: default_content
                            })
                        }},
                    })
                }
            } else {
                error(
                    "Type arguments are supported only for Option, Switch, Vec",
                    &ty_last,
                )
            }
        }
        _ => error("Unsupported type", &ty),
    }
}
