use crate::{
    basic_components::{Button, ButtonType, IconButton},
    translation::use_translation,
};
use alvr_common::{data::SessionDesc, prelude::*};
use yew::{html, Callback, Properties};
use yew_functional::{function_component, use_context};

#[function_component(Connections)]
pub fn connections() -> Html {
    let session = use_context::<SessionDesc>().unwrap();
    let t = use_translation().get_attributes("connections");

    let new_clients = session
        .client_connections
        .iter()
        .filter(|(_, v)| !v.trusted);
    let trusted_clients = session.client_connections.iter().filter(|(_, v)| v.trusted);

    html! {
        <div>
            <section class="px-4 py-3">
                <div class="py-2 font-semibold text-gray-600 text-xl">
                    {t["devices"].clone()}
                </div>
                <div class="flex gap-8 flex-wrap py-4">
                    {
                        if !session.client_connections.is_empty() {
                            html! {
                                <>
                                    {
                                        for new_clients.map(|(hostname, connection)| html! {
                                            <Client
                                                display_name=&connection.display_name
                                                hostname=hostname.to_string()
                                                trusted=false
                                            />
                                        })
                                    }
                                    {
                                        for trusted_clients.map(|(hostname, connection)| html! {
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
                                    {t["no-devices"].clone()}
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
    let t = use_translation().get_attributes("connections");

    let on_click = {
        info!("Hostname: {}", "hostname");
        Callback::from(move |_| ())
    };

    let on_trust_click = {
        let hostname = hostname.clone();
        Callback::from(move |_| {
            let hostname = hostname.clone();
            info!("trust: {}", hostname);
            wasm_bindgen_futures::spawn_local(async move {
                reqwest::Client::new()
                    .post(format!("{}/api/client/trust", crate::get_base_url()))
                    .json(&(hostname.clone(), None::<()>))
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
            info!("remove: {}", hostname);
            wasm_bindgen_futures::spawn_local(async move {
                reqwest::Client::new()
                    .post(format!("{}/api/client/remove", crate::get_base_url()))
                    .json(&(hostname.clone(), None::<()>))
                    .send()
                    .await
                    .unwrap();
            });
        })
    };
    html! {
        <div
            class=format!(
                "flex-1 min-w-56 max-w-sm p-4
                shadow-md rounded-lg bg-white border-l-8 transform transition
                hover:shadow-lg hover:-translate-y-1 {}",
                if *trusted {
                    "border-green-500"
                } else {
                    "border-blue-600"
                }
            )
        >
            <h6 class="uppercase text-xs text-gray-400 font-medium">
                {
                    if *trusted {
                        html! {t["trusted-device"].clone()}
                    } else {
                        html! {t["new-device"].clone()}
                    }
                }
            </h6>
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
                                    icon_cls="fas fa-cog"
                                    on_click=on_click
                                    class="hover:bg-blue-500 hover:text-white"
                                />
                                <IconButton
                                    icon_cls="fas fa-trash"
                                    on_click=on_remove_click
                                    class="hover:bg-red-500 hover:text-white"
                                />
                            </>
                        }
                    } else {
                        html! {
                            <Button
                                on_click=on_trust_click
                                button_type=ButtonType::Primary
                            >
                                {t["trust"].clone()}
                            </Button>
                        }
                    }
                }
            </div>
        </div>
    }
}
