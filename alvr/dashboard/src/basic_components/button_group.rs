use yew::{html, Callback, Properties};
use yew_functional::{function_component, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub options: Vec<String>,
    pub selected: String,
    pub on_select: Callback<String>,
}

#[function_component(ButtonGroup)]
pub fn button_group(props: &Props) -> Html {
    let selected = props.selected.clone();
    let on_select = props.on_select.clone();

    let (option, set_option) = use_state(move || selected);

    let on_select = Callback::from(move |o: String| {
        set_option(o.clone());
        on_select.emit(o);
    });

    html! {
        <div class="btn-group" role="group">
            {props.options.iter().map(|o| {
                let id = crate::get_id();
                let on_click = {
                    let o = o.clone();
                    let on_select = on_select.clone();
                    Callback::from(move |_| on_select.emit(o.clone()))
                };
                html! {
                    < key=o.clone()>
                        <input
                            id=id
                            type="radio"
                            class="btn-check"
                            checked=*o==*option
                            onclick=on_click
                        />
                        <label for=id class="btn btn-outline-primary">{o.clone()}</label>
                    </>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
