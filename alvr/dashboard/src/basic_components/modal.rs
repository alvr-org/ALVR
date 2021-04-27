use super::Button;
use crate::translation::use_trans;
use yew::{html, Callback, Children, Properties};
use yew_functional::function_component;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub title: String,
    pub content: String,
    pub on_ok: Callback<()>,
    pub on_cancel: Callback<()>,
    pub on_do_not_ask_again_toggle: Callback<bool>,
    pub children: Children,
}

#[function_component(Modal)]
pub fn modal(props: &Props) -> Html {
    let id = crate::get_id();

    let on_cancel = {
        let on_cancel = props.on_cancel.clone();
        Callback::from(move |_| on_cancel.emit(()))
    };

    let cross_icon = html! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-8 w-8 text-gray-400"
            viewBox="0 0 20 20"
            fill="currentColor"
        >
            <path
                fill-rule="evenodd"
                d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414
                10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0
                01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                clip-rule="evenodd"
            />
        </svg>
    };

    html! {
        <div class="fixed left-0 right-0 top-0 bottom-0 flex items-center justify-center z-50">
            // Backdrop
            <div
                class="absolute w-full h-full bg-gray-500 bg-opacity-70 z-0"
                onclick=on_cancel.clone()
            />
            <div class="bg-white rounded-lg shadow-lg px-4 py-4 z-10 w-full max-w-5xl mx-8 my-8">
                // Header
                <div class="flex justify-between px-2">
                    <h2 class="text-2xl font-medium text-gray-600">{props.title.clone()}</h2>
                    <div onclick=on_cancel.clone()>
                        {cross_icon}
                    </div>
                </div>
                // Content
                <div class="mt-4 p-2">
                    <p class="text-gray-900">
                        {props.content.clone()}
                    </p>
                </div>
                // Actions/footer
                <div class="flex p-2">
                    <input id=id type="checkbox" />
                    <label for=id class="font-medium">{use_trans("ok")}</label>
                    <div class="flex-grow" />
                    <Button on_click=props.on_cancel.clone()>{use_trans("cancel")}</Button>
                    <Button on_click=props.on_ok.clone()>{use_trans("ok")}</Button>
                </div>
            </div>
        </div>
    }
}
