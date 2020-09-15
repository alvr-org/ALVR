use crate::*;
use alvr_common::{data::*, logging::*, sockets::*, *};
use settings_schema::Switch;
use std::time::{Duration, Instant};
use tokio::time;

const STATISTICS_SEND_INTERVAL: Duration = Duration::from_secs(1);
const INPUT_SEND_INTERVAL: Duration = Duration::from_millis(8);

async fn try_connect(
    headset_info: &HeadsetInfoPacket,
    private_identity: &PrivateIdentity,
    on_pause_notifier: broadcast::Sender<()>,
) -> StrResult {
    let (mut control_socket, client_config) = trace_err!(
        ControlSocket::connect_to_server(
            &headset_info,
            private_identity.hostname.clone(),
            private_identity.certificate_pem.clone(),
        )
        .await
    )?;

    // todo: go through session representation. this requires settings -> session representation
    // conversion code
    let settings = trace_err!(serde_json::from_value::<Settings>(client_config.settings))?;

    let mut stream_socket = StreamSocket::connect_to_server(
        control_socket.peer_ip(),
        settings.connection.stream_port,
        private_identity.certificate_pem.clone(),
        private_identity.key_pem.clone(),
        settings.connection.stream_socket_config,
    )
    .await?;

    let input_sender = stream_socket
        .request_stream::<InputPacket>(StreamId::Input, settings.headset.tracking_stream_mode)
        .await?;

    let microphone_sender = if settings.audio.microphone {
        Some(
            stream_socket
                .request_stream::<AudioPacket>(
                    StreamId::Audio,
                    settings.audio.microphone_stream_mode,
                )
                .await?,
        )
    } else {
        None
    };

    let mut video_receiver = stream_socket
        .subscribe_to_stream::<VideoPacket>(StreamId::Video())
        .await?;
    let mut on_pause_receiver = on_pause_notifier.subscribe();
    tokio::spawn(async move {
        loop {
            let packet = tokio::select! {
                Ok(packet) = video_receiver.recv() => packet,
                _ = on_pause_receiver.recv() => break,
                else => break,
            };

            // todo
        }
    });

    if matches!(settings.audio.game_audio, Switch::Enabled(_)) {
        let mut audio_receiver = stream_socket
            .subscribe_to_stream::<AudioPacket>(StreamId::Audio)
            .await?;
        let mut on_pause_receiver = on_pause_notifier.subscribe();
        tokio::spawn(async move {
            loop {
                let packet = tokio::select! {
                    Ok(packet) = audio_receiver.recv() => packet,
                    _ = on_pause_receiver.recv() => break,
                    else => break,
                };

                // todo
            }
        });
    }

    let mut haptics_receiver = stream_socket
        .subscribe_to_stream::<HapticsPacket>(StreamId::Haptics)
        .await?;
    let mut on_pause_receiver = on_pause_notifier.subscribe();
    tokio::spawn(async move {
        loop {
            let packet = tokio::select! {
                Ok(packet) = haptics_receiver.recv() => packet,
                _ = on_pause_receiver.recv() => break,
                else => break,
            };

            // todo
        }
    });

    // todo: send guardian here

    let last_statistics_send_time = Instant::now() - STATISTICS_SEND_INTERVAL;
    let last_input_time = Instant::now();
    loop {
        let input_loop_deadline = time::Instant::now() + INPUT_SEND_INTERVAL;

        // todo: send input

        // todo: maybe send microphone

        if Instant::now() - last_statistics_send_time > STATISTICS_SEND_INTERVAL {
            let stats = STATISTICS.lock().get();
            trace_err!(
                control_socket
                    .send(ClientControlPacket::Statistics(stats))
                    .await
            )?;
        }

        time::delay_until(input_loop_deadline).await;
    }
}

pub async fn connection_loop(
    headset_info: HeadsetInfoPacket,
    private_identity: PrivateIdentity,
    on_pause_notifier: broadcast::Sender<()>,
) {
    loop {
        show_err(try_connect(&headset_info, &private_identity, on_pause_notifier.clone()).await)
            .ok();
    }
}
