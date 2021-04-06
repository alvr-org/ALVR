use yew::{html, Component, ComponentLink, Html, ShouldRender};

pub struct Dashboard {
    link: ComponentLink<Self>,
    label: String,
}

impl Component for Dashboard {
    type Message = ();
    type Properties = ();

    fn create(_props: (), link: ComponentLink<Self>) -> Self {
        Self {
            link,
            label: "Hello".into(),
        }
    }

    fn update(&mut self, _msg: ()) -> ShouldRender {
        self.label = "world".into();

        true
    }

    fn change(&mut self, _props: ()) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <button type="button" class="btm btn-primary" onclick=self.link.callback(|_| ())>
                {&self.label}
            </button>
        }
    }
}
