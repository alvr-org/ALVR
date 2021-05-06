use crate::{
    basic_components::{
        Button, ButtonGroup, ButtonType, Select, Slider, Switch, TextField, UpDown,
    },
    components::{About, Connections, Installation, Logs, Settings, Statistics},
    translation::use_translation,
};
use yew::{html, Callback, Properties};
use yew_functional::{function_component, use_state};

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let selected_tab_handle = use_state(|| "connections".to_owned());

    let on_tab_click = {
        let selected_tab_handle = selected_tab_handle.clone();
        Callback::from(move |name| selected_tab_handle.set(name))
    };

    let translation_on_click = Callback::from(move |_| {});

    html! {
        <div class="flex flex-col h-screen">
            <div class="flex-grow flex items-stretch bg-gray-100 select-none h-full">
                <aside class="bg-gray-200">
                    <nav class="flex flex-col items-stretch h-full py-4 space-y-2">
                        <TabEntry
                            name="connections"
                            icon="fas fa-plug"
                            on_click=on_tab_click.clone()
                            selected=*selected_tab_handle == "connections"
                        />
                        <TabEntry
                            name="statistics"
                            icon="fas fa-chart-bar"
                            on_click=on_tab_click.clone()
                            selected=*selected_tab_handle == "statistics"
                        />
                        // <TabEntry
                        //     name="presets"
                        //     icon="fas fa-th-large"
                        //     on_click=on_tab_click.clone()
                        //     selected=*selected_tab_handle == "presets"
                        // />
                        <TabEntry
                            name="settings"
                            icon="fas fa-cog"
                            on_click=on_tab_click.clone()
                            selected=*selected_tab_handle == "settings"
                        />
                        <TabEntry
                            name="installation"
                            icon="fas fa-hdd"
                            on_click=on_tab_click.clone()
                            selected=*selected_tab_handle == "installation"
                        />
                        <TabEntry
                            name="logs"
                            icon="fas fa-th-list"
                            on_click=on_tab_click.clone()
                            selected=*selected_tab_handle == "logs"
                        />
                        <TabEntry
                            name="about"
                            icon="fas fa-info-circle"
                            on_click=on_tab_click.clone()
                            selected=*selected_tab_handle == "about"
                        />
                        <TabEntry
                            name="test"
                            icon="fas fa-asterisk"
                            on_click=on_tab_click.clone()
                            selected=*selected_tab_handle == "test"
                        />
                        <div class="flex-auto" />
                        <TabEntry
                            name="language"
                            icon="fas fa-globe"
                            on_click=translation_on_click
                            selected=false
                        />
                    </nav>
                </aside>
                <div class="flex-grow h-full overflow-y-auto">
                    <div hidden=*selected_tab_handle != "connections">
                        <Connections />
                    </div>
                    <div hidden=*selected_tab_handle != "statistics">
                        <Statistics />
                    </div>
                    <div hidden=*selected_tab_handle != "settings">
                        <Settings />
                    </div>
                    <div hidden=*selected_tab_handle != "installation">
                        <Installation />
                    </div>
                    <div hidden=*selected_tab_handle != "logs">
                        <Logs />
                    </div>
                    <div hidden=*selected_tab_handle != "about">
                        <About />
                    </div>
                    <div hidden=*selected_tab_handle != "test">
                        <Test />
                    </div>
                </div>
            </div>
            // <div></div> // todo notifications
        </div>
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct MenuIconProps {
    pub name: String,
    pub icon: String,
    pub on_click: Callback<String>,
    pub selected: bool,
}

#[function_component(TabEntry)]
pub fn tab_entry(props: &MenuIconProps) -> Html {
    let on_click = {
        let on_click = props.on_click.clone();
        let name = props.name.clone();
        Callback::from(move |_| on_click.emit(name.clone()))
    };

    html! {
        <div
            class=format!(
                "mr-3
                rounded-r-lg
                cursor-pointer
                transition-color transition-transform
                {}",
                if props.selected {
                    "bg-blue-700 hover:bg-blue-800
                    shadow-md text-white"
                } else {
                    "transform -translate-x-2 hover:-translate-x-1
                    bg-gray-300 hover:bg-gray-400"
                },
            )
            onclick=on_click
        >
            <div
                class=format!(
                    "flex w-full h-full py-1 pr-5
                    transition-transform
                    {}",
                    if props.selected {
                        ""
                    } else {
                        // exact opposite of parent
                        "transform translate-x-2 hover:translate-x-1"
                    }
                )
            >
                <div class="w-10 flex justify-center items-center">
                    <i
                        class=format!(
                            "opacity-75 {} {}",
                            if props.selected { "opacity-90" } else { "" },
                            props.icon.clone()
                        )
                    />
                </div>
                <div class="font-medium">{use_translation().get(&props.name)}</div>
            </div>
        </div>
    }
}

#[function_component(Test)]
pub fn test() -> Html {
    let label_handle = use_state(|| "Hello".to_owned());

    let on_click = {
        let label_handle = label_handle.clone();
        Callback::from(move |_| label_handle.set(format!("{} world", *label_handle)))
    };

    let default_string = use_translation().get("default");

    let switch_on_click = Callback::from(move |_| ());

    let slider_on_change = Callback::from(move |_| ());

    let on_select = Callback::from(move |_| ());

    let text_field_on_focus_lost = Callback::from(move |_| ());

    let up_down_on_step = Callback::from(move |_| ());

    html! {
        <div class="px-4 py-3">
            <div class="flex flex-col space-y-2 items-start">
                <Button on_click=on_click.clone() button_type=ButtonType::None>
                    {(*label_handle).clone()}
                </Button>
                <Button on_click=on_click.clone() button_type=ButtonType::Primary>
                    {(*label_handle).clone()}
                </Button>
                <Button on_click=on_click.clone() button_type=ButtonType::Secondary>
                    {(*label_handle).clone()}
                </Button>
                <Button on_click=on_click button_type=ButtonType::Danger>
                    {(*label_handle).clone()}
                </Button>
            </div>
            <Switch on_click=switch_on_click checked=true/>
            <Slider value="0" default="30" min="-1" max="40" step="0.5" on_change=slider_on_change/>
            <ButtonGroup
                options=vec!["hello1".into(), "hello2".into()]
                selected="hello1"
                on_select=on_select.clone()
            />
            <Select
                options=vec!["hello1".into(), "hello2".into()]
                selected="hello1"
                on_select=on_select
            />
            <div class="space-y-2">
                <TextField
                    value=default_string.clone()
                    on_focus_lost=text_field_on_focus_lost.clone()
                />
                <TextField
                    label="Hi there"
                    value=default_string
                    on_focus_lost=text_field_on_focus_lost.clone()
                />
            </div>
            <div class="py-2 space-y-2">
                <UpDown
                    label="Bitrate"
                    value="123"
                    on_focus_lost=text_field_on_focus_lost.clone()
                    on_step_down=up_down_on_step.clone()
                    on_step_up=up_down_on_step.clone()
                />
                <UpDown
                    value="123"
                    on_focus_lost=text_field_on_focus_lost
                    on_step_down=up_down_on_step.clone()
                    on_step_up=up_down_on_step
                />
            </div>
        </div>
    }
}

// https://play.tailwindcss.com/a02WW4bd69
