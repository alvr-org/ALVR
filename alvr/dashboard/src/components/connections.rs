use crate::{
    basic_components::{
        Button, ButtonGroup, ButtonType, IconButton, Select, Slider, Switch, TextField, UpDown,
    },
    translation::use_trans,
};
use alvr_common::data::{ClientConnectionDesc, SessionDesc};
use std::{collections::HashMap, rc::Rc};
use yew::{html, Callback, Properties};
use yew_functional::function_component;

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub session: Rc<SessionDesc>,
}

#[function_component(Connections)]
pub fn connections(props: &Props) -> Html {
    // let on_click = Callback::from(move |_| ());
    let session = props.session.clone();
    for (key, value) in &props.session.client_connections {
        log::info!("{:?}", key);
    }
    let new_clients: &HashMap<&String, &ClientConnectionDesc> = &props
        .session
        .client_connections
        .iter()
        .filter(|(k, v)| v.trusted == false)
        .collect();
    let trusted_clients: &HashMap<&String, &ClientConnectionDesc> = &props
        .session
        .client_connections
        .iter()
        .filter(|(k, v)| v.trusted == true)
        .collect();
    html! {
        <div>
            <section class="px-4 py-3">
                <div class="py-2 font-semibold text-gray-600 text-xl">
                    {"Devices"}
                </div>
                <div class="flex gap-8 flex-wrap py-4">
                    {
                        if new_clients.len() > 0 || trusted_clients.len() > 0 {
                            html! {
                                <>
                                    {
                                        for new_clients.iter().map(|(hostname, connection)| html! {
                                            <Client
                                                display_name=&connection.display_name
                                                hostname=hostname.to_string()
                                                trusted=false
                                            />
                                        })
                                    }
                                    {
                                        for trusted_clients.iter().map(|(hostname, connection)| html! {
                                            <Client
                                                display_name=&connection.display_name
                                                hostname=hostname.to_string()
                                                trusted=true
                                            />
                                        })
                                    }
                                </>
                            }
                        } else {
                            html! {
                                <div
                                    class=format!(
                                        "flex-1 flex items-center justify-center py-4 px-1 {}",
                                        "text-gray-500 font-semibold text-lg"
                                    )
                                >
                                    {"No Devices"}
                                    // TODO: add link to troubleshooting page if no devices
                                </div>
                            }
                        }
                    }
                </div>
            </section>
        </div>
    }
}

#[derive(Properties, Clone, PartialEq)]
pub struct ClientProps {
    pub display_name: String,

    pub hostname: String,

    pub trusted: bool,
}

#[function_component(Client)]
pub fn client(
    ClientProps {
        display_name,
        hostname,
        trusted,
    }: &ClientProps,
) -> Html {
    let on_click = {
        log::info!("Hostname: {}", "hostname");
        Callback::from(move |_| ())
    };

    let on_trust_click = {
        let hostname = hostname.clone();
        Callback::from(move |_| {
            let hostname = hostname.clone();
            log::info!("trust: {}", hostname);
            wasm_bindgen_futures::spawn_local(async move {
                let _ = reqwest::Client::new()
                    .post(format!("{}/api/client/trust", crate::get_base_url()))
                    .json(&(hostname.clone(), None::<String>))
                    .send()
                    .await
                    .unwrap();
            });
        })
    };
    let on_remove_click = {
        let hostname = hostname.clone();
        Callback::from(move |_| {
            let hostname = hostname.clone();
            log::info!("remove: {}", hostname);
            wasm_bindgen_futures::spawn_local(async move {
                let _ = reqwest::Client::new()
                    .post(format!("{}/api/client/remove", crate::get_base_url()))
                    .json(&(hostname.clone(), None::<String>))
                    .send()
                    .await
                    .unwrap();
            });
        })
    };
    html! {
        <div
            class=format!(
                "flex-1 min-w-56 max-w-sm p-4 {} {} {}",
                "shadow-md rounded-lg bg-white border-l-8 transform transition",
                "hover:shadow-lg hover:-translate-y-1",
                if *trusted {
                    "border-green-500"
                } else {
                    "border-blue-600"
                })
        >
        {
            if *trusted {
                html! {
                    <h6
                        class="uppercase text-xs text-gray-400 font-medium"
                    >
                        {"Trusted"}
                    </h6>
                }
            } else {
                html! {
                    <h6
                        class="uppercase text-xs text-gray-400 font-medium"
                    >
                        {"New!"}
                    </h6>
                }
            }
        }
        <h4
            class="text-gray-700 font-medium text-xl mt-1"
        >
            {display_name}
        </h4>
        <div class="mt-4">
            <div class="text-gray-700 font-medium">
                {"Hostname"}
            </div>
            <div class="text-gray-800 text-lg">
                {hostname}
            </div>
        </div>
        <div class="flex justify-end space-x-2 mt-4">
            {
                if *trusted {
                    html! {
                        <>
                            <IconButton
                                icon_cls="fas fa-trash"
                                on_click=on_remove_click
                                button_type=ButtonType::None
                                class="hover:bg-red-500 hover:text-white"
                            />
                            <IconButton
                                icon_cls="fas fa-cog"
                                on_click=on_click
                                button_type=ButtonType::None
                                class="hover:bg-blue-500 hover:text-white"
                            />
                        </>
                    }
                } else {
                    html! {
                        <Button
                            on_click=on_trust_click
                            button_type=ButtonType::Primary
                        >
                            {"Trust"}
                        </Button>
                    }
                }
            }
        </div>
    </div>
    }
}
