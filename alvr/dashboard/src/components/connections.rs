use crate::{
    basic_components::{
        Button, ButtonGroup, ButtonType, Select, Slider, Switch, TextField, UpDown,
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
        <div class="bg-white h-full">
            <div class="p-10 pb-0 gap-5">
                <div class="px-3 py-2 font-medium text-gray-700">
                    {"New Clients"}
                </div>
                <div class="px-3 py-2">
                    {if new_clients.len() > 0 { html! {
                        { for new_clients.iter().map(|(hostname, connection)| html! {
                            <Client display_name=&connection.display_name hostname=hostname.to_string() trusted=false> </Client>
                        }) }
                    }} else {
                        html! {
                            <div
                              class="flex text-gray-800 border-l-4 border-red-500 px-3 shadow py-2 bg-gray-50 rounded"
                            >
                              <div>
                                {"No new clients!"}
                              </div>
                            </div>
                        }
                    }}
                </div>
            </div>
            <div class="p-10 pb-0 gap-5">
                <div class="px-3 py-2 font-medium text-gray-700">
                    {"Trusted Clients"}
                </div>
                <div class="px-3 py-2">
                    {if trusted_clients.len() > 0 { html! {
                        { for trusted_clients.iter().map(|(hostname, connection)| html! {
                            <Client display_name=&connection.display_name hostname=hostname.to_string() trusted=true> </Client>
                        }) }
                    }} else {
                        html! {
                            <div
                            class="flex text-gray-800 border-l-4 border-red-500 px-3 shadow py-2 bg-gray-50 rounded"
                            >
                                <div>
                                    {"You haven't trusted any clients yet!"}
                                </div>
                            </div>
                        }
                    }}
                </div>
            </div>
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
          class=format!("flex text-gray-800 border-l-4 {} px-3 shadow py-2 bg-gray-50 rounded", if *trusted {"border-green-400"} else {"border-blue-500"})
        >
          <div>
            {display_name}
            <p class="ml-3">
                {format!("Hostname: {}", hostname)}
            </p>
          </div>
            { if *trusted {
                html! {
                    <>
                        <Button on_click=on_click.clone() button_type=ButtonType::Primary class="ml-auto h-9 self-center">{"Configure"}</Button>
                        <Button on_click=on_remove_click button_type=ButtonType::Primary class="ml-1 h-9 self-center">{"Remove"}</Button>
                    </>
                }
            } else {
                html! {
                    <Button on_click=on_trust_click button_type=ButtonType::Primary class="ml-auto h-9 self-center">{"Trust"}</Button>
                }
            } }
        </div>
    }
}
