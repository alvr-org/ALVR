use super::SettingProps;
use serde_json as json;
use settings_schema::{SchemaSwitch, SwitchDefault};
use yew::{html, Callback, Html};
use yew_functional::function_component;

#[function_component(SwitchControl)]
pub fn switch_control(props: &SettingProps<SchemaSwitch, SwitchDefault<json::Value>>) -> Html {
    html!("switch control")
}

pub fn switch_container(
    schema: SchemaSwitch,
    session: SwitchDefault<json::Value>,
    set_session: Callback<SwitchDefault<json::Value>>,
    advanced: bool,
) -> Option<Html> {
    if session.enabled && (advanced || !schema.content_advanced) {
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
