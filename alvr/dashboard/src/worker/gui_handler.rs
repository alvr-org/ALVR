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

            let session = match response.json::<alvr_session::SessionDesc>().await {
                Ok(session) => session,
                Err(why) => {
                    println!("Error parsing session JSON: {}", why);
                    // Err returns are reserved for connectivity errors
                    return Ok(false);
                }
            };

            // Discarded as the receiving end will always be valid, and when it is not the dashboard is shutting down anyway
            let _ = tx1.send(WorkerMsg::SessionResponse(session));
            false
        }
        GuiMsg::GetDrivers => {
            get_drivers(client, tx1).await?;
            false
        }
        GuiMsg::Dashboard(response) => match response {
            DashboardResponse::SessionUpdated(session) => {
                let text = serde_json::to_string(&session).unwrap();
                client
                    .get(format!("{}/api/session/store", BASE_URL))
                    .body(format!("{{\"session\": {}}}", text))
                    .send()
                    .await?;
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

    let vec: Vec<String> = match response.json().await {
        Ok(vec) => vec,
        Err(why) => {
            println!("Error parsing driver list JSON: {}", why);
            // We return Ok(()) here as an Err variant is used to signal being offline
            return Ok(());
        }
    };

    // If this errors out, the GUI thread has already exited anyway and the worker will as well so it is safe to discard the error
    let _ = tx1.send(WorkerMsg::DriverResponse(vec));

    Ok(())
}
