use dashboard::dashboard::{DashboardResponse, DriverResponse, FirewallRulesResponse};

use crate::{GuiMsg, WorkerMsg};

use super::BASE_URL;

pub async fn handle_msg(
    msg: GuiMsg,
    client: &reqwest::Client,
    tx1: &std::sync::mpsc::Sender<WorkerMsg>,
) -> reqwest::Result<bool> {
    Ok(match msg {
        GuiMsg::Quit => true,
        GuiMsg::GetSession => {
            let response = client
                .get(format!("{}/api/session/load", BASE_URL))
                .send()
                .await?;

            tx1.send(WorkerMsg::SessionResponse(
                response.json::<alvr_session::SessionDesc>().await.unwrap(),
            ))
            .unwrap();
            false
        }
        GuiMsg::GetDrivers => {
            get_drivers(client, tx1).await?;
            false
        }
        GuiMsg::Dashboard(response) => match response {
            DashboardResponse::SessionUpdated(session) => {
                let text = serde_json::to_string(&session).unwrap();
                let response = client
                    .get(format!("{}/api/session/store", BASE_URL))
                    .body(format!("{{\"session\": {}}}", text))
                    .send()
                    .await?;
                if !response.status().is_success() {
                    println!(
                        "HTTP request returned an error: {:?}",
                        response.error_for_status().unwrap()
                    );
                }
                false
            }
            DashboardResponse::RestartSteamVR => {
                client
                    .get(format!("{}/restart-steamvr", BASE_URL))
                    .send()
                    .await?;
                false
            }
            DashboardResponse::Driver(driver) => match driver {
                DriverResponse::RegisterAlvr => {
                    let response = client
                        .get(format!("{}/api/driver/register", BASE_URL))
                        .send()
                        .await?;

                    println!("{}", response.status());

                    get_drivers(client, tx1).await?;
                    false
                }
                DriverResponse::Unregister(path) => {
                    let response = client
                        .get(format!("{}/api/driver/unregister", BASE_URL))
                        .body(format!(r#""{}""#, path))
                        .send()
                        .await?;
                    println!("{}", response.status());
                    get_drivers(client, tx1).await?;
                    false
                }
            },
            DashboardResponse::Firewall(firewall) => match firewall {
                FirewallRulesResponse::Add => {
                    client
                        .get(format!("{}/api/firewall-rules/add", BASE_URL))
                        .send()
                        .await?;
                    false
                }
                FirewallRulesResponse::Remove => {
                    client
                        .get(format!("{}/api/firewall-rules/remove", BASE_URL))
                        .send()
                        .await?;
                    false
                }
            },
            _ => false,
        },
    })
}

// Some functions to reduce code duplication
async fn get_drivers(
    client: &reqwest::Client,
    tx1: &std::sync::mpsc::Sender<WorkerMsg>,
) -> reqwest::Result<()> {
    let response = client
        .get(format!("{}/api/driver/list", BASE_URL))
        .send()
        .await?;

    let vec: Vec<String> = response.json().await.unwrap();

    tx1.send(WorkerMsg::DriverResponse(vec)).unwrap();

    Ok(())
}
