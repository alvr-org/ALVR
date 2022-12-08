use alvr_client_core::ClientEvent;
use alvr_events::ButtonValue;
use alvr_session::OculusFovetionLevel;
use alvr_sockets::{DeviceMotion, Fov, Tracking};
use std::time::Duration;

fn main() {
    let tracking = serde_json::to_string_pretty(&Tracking {
        target_timestamp: Duration::ZERO,
        device_motions: vec![(0, DeviceMotion::default())],
        left_hand_skeleton: None,
        right_hand_skeleton: Some(Default::default()),
    })
    .unwrap();

    let client_event = serde_json::to_string_pretty(&ClientEvent::StreamingStarted {
        view_resolution: Default::default(),
        fps: 0.0,
        oculus_foveation_level: OculusFovetionLevel::None,
        dynamic_oculus_foveation: false,
        extra_latency: false,
        controller_prediction_multiplier: 0.0,
    })
    .unwrap();

    let fov = serde_json::to_string_pretty(&Fov::default()).unwrap();

    let button_value = serde_json::to_string_pretty(&ButtonValue::Scalar(0.0)).unwrap();

    println!("Tracking: {tracking}\n\nClientEvent: {client_event}\n\nFov: {fov}\n\nButtonValue: {button_value}");
}
