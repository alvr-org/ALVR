use super::SettingProps;
use serde_json as json;
use settings_schema::{OptionalDefault, SchemaOptional};
use yew::{html, Callback, Html};
use yew_functional::function_component;

#[function_component(OptionalControl)]
pub fn optional_control(
    props: &SettingProps<SchemaOptional, OptionalDefault<json::Value>>,
) -> Html {
    html!("optional control")
}

pub fn optional_container(
    schema: SchemaOptional,
    session: OptionalDefault<json::Value>,
    set_session: Callback<OptionalDefault<json::Value>>,
    advanced: bool,
) -> Option<Html> {
    if session.set {
        super::setting_container(
            *schema.content,
            session.content.clone(),
            Callback::from(move |child_session| {
                let mut session = session.clone();
                session.content = child_session;
                set_session.emit(session);
            }),
            advanced,
        )
    } else {
        None
    }
}
