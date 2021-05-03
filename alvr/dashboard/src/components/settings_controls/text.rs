use super::{Reset, SettingProps};
use crate::basic_components::TextField;
use yew::html;
use yew_functional::function_component;

#[function_component(Text)]
pub fn text(props: &SettingProps<String, String>) -> Html {
    html! {
        <div class="flex gap-1">
            <TextField
                value=props.session.clone()
                on_focus_lost=props.set_session.clone()
            />
            <Reset<String> default=props.schema.clone() set_default=props.set_session.clone() />
        </div>
    }
}
