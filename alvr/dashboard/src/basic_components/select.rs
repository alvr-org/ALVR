use yew::{html, html::ChangeData, Callback, Properties};
use yew_functional::{function_component, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub options: Vec<String>,
    pub selected: String,
    pub on_select: Callback<String>,
}

#[function_component(Select)]
pub fn select(props: &Props) -> Html {
    let selected = props.selected.clone();
    let on_select = props.on_select.clone();

    let option_handle = use_state(move || selected);

    let on_change = {
        let option_handle = option_handle.clone();
        Callback::from(move |data: ChangeData| {
            if let ChangeData::Select(element) = data {
                let option = element.value();
                option_handle.set(option.clone());
                on_select.emit(option);
            }
        })
    };

    html! {
        <select class="form-select" onchange=on_change>
            {props.options.iter().map(|option| html! {
                <option
                    key=option.clone()
                    value=option
                    selected=*option==*option_handle
                >
                    {option}
                </option>
            }).collect::<Vec<_>>()}
        </select>
    }
}
