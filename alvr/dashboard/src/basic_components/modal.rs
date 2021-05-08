use super::Button;
use crate::translation::use_translation;
use yew::{html, Callback, Children, Properties};
use yew_functional::{function_component, use_state};

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub use_do_not_ask_again: bool,
    pub on_ok: Callback<bool>, // returns if should do not ask again
    pub on_cancel: Callback<()>,
    pub children: Children,
}

#[function_component(Modal)]
pub fn modal(props: &Props) -> Html {
    let t = use_translation();

    let id = crate::get_id();

    let do_not_ask_again = use_state(|| false);

    let on_ok = {
        let on_ok = props.on_ok.clone();
        let do_not_ask_again = do_not_ask_again.clone();
        Callback::from(move |_| on_ok.emit(*do_not_ask_again))
    };

    html! {
        <div class="fixed left-0 right-0 top-0 bottom-0 flex items-center justify-center z-50">
            // Backdrop
            <div
                class="absolute w-full h-full bg-gray-500 bg-opacity-70 z-0"
                onclick={
                    let on_cancel = props.on_cancel.clone();
                    Callback::from(move |_| on_cancel.emit(()))
                }
            />
            <div class="bg-white rounded-lg shadow-lg px-4 py-4 z-10 w-full max-w-5xl mx-8 my-8">
                // Content
                <div class="mt-4 p-2 text-gray-900">
                    {props.children.clone()}
                </div>
                // Actions/footer
                <div class="flex p-2">
                    {
                        if props.use_do_not_ask_again {
                            html! {
                                <>
                                    <input
                                        id=id
                                        type="checkbox"
                                        checked=*do_not_ask_again
                                        onclick=Callback::from(move |_| {
                                            do_not_ask_again.set(!*do_not_ask_again)
                                        })
                                    />
                                    <label for=id class="font-medium">
                                        {t.get("do-not-ask-again")}
                                    </label>
                                </>
                            }
                        } else {
                            html!()
                        }
                    }
                    <div class="flex-grow" />
                    <Button on_click=props.on_cancel.clone()>{t.get("cancel")}</Button>
                    <Button on_click=on_ok>{t.get("ok")}</Button>
                </div>
            </div>
        </div>
    }
}
