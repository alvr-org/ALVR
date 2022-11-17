mod gui_handler;

use futures_util::StreamExt;
use std::time::Duration;
use tokio::sync::{
    broadcast,
    mpsc::{self, error::TryRecvError},
};
use tokio_tungstenite::{connect_async, tungstenite};

use crate::{GuiMsg, WorkerMsg};

const BASE_URL: &str = "http://localhost:8082";
const BASE_WS_URL: &str = "ws://localhost:8082";

pub fn http_thread(
    tx1: std::sync::mpsc::Sender<WorkerMsg>,
    rx2: std::sync::mpsc::Receiver<GuiMsg>,
) {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let client = reqwest::Client::builder().build().unwrap();

        // Communication with the event thread
        let (broadcast_tx, _) = broadcast::channel(1);
        let mut event_rx = None;

        let mut connected = false;

        'main: loop {
            match client.get(BASE_URL).send().await {
                Ok(_) => {
                    // When successfully connected let's (re)create the event stream
                    if !connected {
                        let (event_tx, _event_rx) = mpsc::channel::<alvr_events::Event>(1);
                        tokio::task::spawn(websocket_task(
                            url::Url::parse(&format!("{}/api/events", BASE_WS_URL)).unwrap(),
                            event_tx,
                            broadcast_tx.subscribe(),
                        ));
                        event_rx = Some(_event_rx);
                        let _ = tx1.send(WorkerMsg::Connected);
                        connected = true;
                    }
                }
                Err(why) => {
                    let _ = broadcast_tx.send(());
                    connected = false;

                    // We still check for the exit signal from the Gui thread
                    for msg in rx2.try_iter() {
                        if let GuiMsg::Quit = msg {
                            break 'main;
                        }
                    }

                    let _ = tx1.send(WorkerMsg::LostConnection(format!("{}", why)));
                }
            }

            // If we are not connected, don't even attempt to continue normal working order
            if !connected {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            loop {
                match event_rx.as_mut().unwrap().try_recv() {
                    Ok(event) => {
                        let _ = tx1.send(WorkerMsg::Event(event));
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(why) => {
                        println!("Error receiving event from event worker: {}", why);
                        break;
                    }
                }
            }

            for msg in rx2.try_iter() {
                match gui_handler::handle_msg(msg, &client, &tx1).await {
                    Ok(quit) => {
                        if quit {
                            break 'main;
                        }
                    }
                    Err(why) => {
                        let _ = broadcast_tx.send(());
                        connected = false;
                        let _ = tx1.send(WorkerMsg::LostConnection(format!("{}", why)));
                    }
                }
            }
            // With each iteration we should sleep to not consume a thread fully
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        // Shutdown the event thread if needed, an error would only mean that the event thread is already dead so we ignore it
        let _ = broadcast_tx.send(());
    });
}
async fn websocket_task<T: serde::de::DeserializeOwned + std::fmt::Debug>(
    url: url::Url,
    sender: tokio::sync::mpsc::Sender<T>,
    mut recv: tokio::sync::broadcast::Receiver<()>,
) {
    // Connect to the event stream, and split it so we can get only the read stream
    let (event_stream, _) = connect_async(url).await.unwrap();
    let (_, event_read) = event_stream.split();

    // The select macro is used to cancel the event task if a shutdown signal is received
    tokio::select! {
        _ = event_read.for_each(|msg| async {
            match msg {
                Ok(
                tungstenite::Message::Text(text)) => {
                    let event = serde_json::from_str::<T>(&text).unwrap();

                    match sender.send(event).await {
                        Ok(_) => (),
                        Err(why) => {
                            println!("Error sending event: {}", why);
                        }
                    }
                }
                Ok(_) => (),
                Err(why) => println!("Error receiving event: {}", why),
            }
        }) => {},
        _ = recv.recv() => {},
    };
}
