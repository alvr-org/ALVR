#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
use alvr_common::{ConnectionError, anyhow::Result, debug, error, parking_lot::Mutex};
use alvr_session::AudioBufferingConfig;
use alvr_sockets::StreamReceiver;
use pipewire::stream::StreamState;
use pipewire::{
    channel::Receiver,
    context::Context,
    keys,
    main_loop::MainLoop,
    properties,
    spa::{
        param::audio::{AudioFormat, AudioInfoRaw},
        pod::{self, Pod, Value, serialize::PodSerializer},
        utils::Direction,
    },
    stream::{Stream, StreamFlags, StreamListener},
};
use std::thread;
use std::time::Duration;
use std::{cmp, collections::VecDeque, io, sync::Arc};

use crate::linux::Terminate;

pub fn play_loop(
    running: impl Fn() -> bool,
    channels_count: u16,
    sample_rate: u32,
    config: AudioBufferingConfig,
    receiver: &mut StreamReceiver<()>,
) -> Result<()> {
    let batch_frames_count = sample_rate as usize * config.batch_ms as usize / 1000;
    let average_buffer_frames_count =
        sample_rate as usize * config.average_buffering_ms as usize / 1000;

    let sample_buffer = Arc::new(Mutex::new(VecDeque::new()));

    let (pw_sender, pw_receiver) = pipewire::channel::channel();
    let receive_samples_buffer_arc = Arc::clone(&sample_buffer);
    let pw_stream_state = Arc::new(Mutex::new(StreamState::Unconnected));
    let pw_stream_state_arc = Arc::clone(&pw_stream_state);

    let thread_handle = microphone_loop_thread(
        channels_count,
        sample_rate,
        pw_receiver,
        pw_stream_state,
        Arc::clone(&sample_buffer),
    );
    while running() {
        let is_microphone_running = is_microphone_running(&running, &pw_stream_state_arc);
        crate::receive_samples_loop(
            is_microphone_running,
            receiver,
            receive_samples_buffer_arc.clone(),
            channels_count as _,
            batch_frames_count,
            average_buffer_frames_count,
        )
        .ok();

        // if we end up here then no consumer is currently connected to the output,
        // so discard audio packets to not cause a buildup
        if matches!(
            receiver.recv(Duration::from_millis(500)),
            Err(ConnectionError::Other(_))
        ) {
            break;
        }
    }

    terminate_pipewire(&pw_sender, thread_handle);
    Ok(())
}

fn terminate_pipewire(
    pw_sender: &pipewire::channel::Sender<Terminate>,
    thread: thread::JoinHandle<()>,
) {
    if pw_sender.send(Terminate).is_err() {
        error!(
            "Couldn't send pipewire termination signal, deinitializing forcefully.
            Restart VR app to reinitialize microphone device."
        );
        unsafe { pipewire::deinit() };
    }

    match thread.join() {
        Ok(()) => debug!("Pipewire microphone thread joined"),
        Err(_) => {
            error!("Couldn't wait for pipewire microphone thread to finish.");
        }
    }
}

fn is_microphone_running(
    running: &impl Fn() -> bool,
    pw_stream_state_arc: &Arc<
        alvr_common::parking_lot::lock_api::Mutex<alvr_common::parking_lot::RawMutex, StreamState>,
    >,
) -> impl Fn() -> bool {
    || {
        pw_stream_state_arc
            .try_lock()
            .is_some_and(|stream_state| *stream_state == StreamState::Streaming && running())
    }
}

fn microphone_loop_thread(
    channels_count: u16,
    sample_rate: u32,
    pw_receiver: pipewire::channel::Receiver<Terminate>,
    pw_stream_state: Arc<
        alvr_common::parking_lot::lock_api::Mutex<alvr_common::parking_lot::RawMutex, StreamState>,
    >,
    pw_loop_buffer_arc: Arc<
        alvr_common::parking_lot::lock_api::Mutex<
            alvr_common::parking_lot::RawMutex,
            VecDeque<f32>,
        >,
    >,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        match pipewire_main_loop(
            pw_stream_state,
            sample_rate,
            channels_count,
            pw_receiver,
            pw_loop_buffer_arc,
        ) {
            Ok(()) => {
                debug!("Pipewire microphone loop exiting");
            }
            Err(e) => {
                error!(
                    "Unhandled pipewire microphone device error, please report it to GitHub: {e}"
                );
            }
        }
    })
}

fn pipewire_main_loop(
    pw_stream_state: Arc<Mutex<StreamState>>,
    sample_rate: u32,
    channels_count: u16,
    pw_receiver: Receiver<Terminate>,
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
) -> Result<(), pipewire::Error> {
    debug!("Starting microphone pw-thread");
    let mainloop = MainLoop::new(None)?;

    let _receiver = pw_receiver.attach(mainloop.as_ref(), {
        let mainloop = mainloop.clone();
        move |_| mainloop.quit()
    });

    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;

    let stream = Stream::new(
        &core,
        "alvr-mic",
        properties::properties! {
            *keys::NODE_NAME => "ALVR Microphone",
            *keys::MEDIA_NAME => "alvr-mic",
            *keys::MEDIA_TYPE => "Audio",
            *keys::MEDIA_CATEGORY => "Playback",
            *keys::MEDIA_CLASS => "Audio/Source",
            *keys::MEDIA_ROLE => "Communication",
        },
    )?;

    let chan_size = std::mem::size_of::<f32>();
    let default_channels_count: usize = channels_count.into();
    // Amount of bytes one full processing will take
    let stride = chan_size * default_channels_count;
    let _listener: StreamListener<f32> = stream
        .add_local_listener()
        .state_changed(move |_, _, _, new_state| {
            *pw_stream_state.lock() = new_state;
        })
        .process(move |stream, _| {
            pipewire_loop_process(sample_buffer.clone(), chan_size, stride, stream);
        })
        .register()?;

    let mut audio_info = AudioInfoRaw::new();
    audio_info.set_format(AudioFormat::F32LE);
    audio_info.set_rate(sample_rate);
    audio_info.set_channels(channels_count.into());

    let values: Vec<u8> = PodSerializer::serialize(
        io::Cursor::new(Vec::new()),
        &Value::Object(pod::Object {
            type_: libspa_sys::SPA_TYPE_OBJECT_Format,
            id: libspa_sys::SPA_PARAM_EnumFormat,
            properties: audio_info.into(),
        }),
    )
    .unwrap()
    .0
    .into_inner();

    let mut params = [Pod::from_bytes(&values).unwrap()];

    stream.connect(
        Direction::Output,
        None,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
        &mut params,
    )?;
    debug!("Prepared microphone pw-thread");

    mainloop.run();
    Ok(())
}

fn pipewire_loop_process(
    sample_buffer: Arc<
        alvr_common::parking_lot::lock_api::Mutex<
            alvr_common::parking_lot::RawMutex,
            VecDeque<f32>,
        >,
    >,
    chan_size: usize,
    stride: usize,
    stream: &pipewire::stream::StreamRef,
) {
    match stream.dequeue_buffer() {
        None => {
            // Nothing is connected to stream, continue
        }
        Some(mut pw_buffer) => {
            let requested_buffer_size = pw_buffer.requested();

            let datas = pw_buffer.datas_mut();
            if datas.is_empty() {
                return;
            }

            let mut total_size = 0;
            let pw_data = &mut datas[0];
            if let Some(slice) = pw_data.data() {
                // How much of slices of out data we will process
                // Get minimum number from what pipewire suggests and maximum possible value by one stride
                let n_frames = cmp::min(requested_buffer_size as usize, slice.len() / stride);
                total_size = n_frames;

                for i in 0..n_frames {
                    let start = i * stride;
                    let end = start + chan_size;
                    let channel = &mut slice[start..end];
                    match sample_buffer.try_lock() {
                        Some(mut buff) => match buff.pop_front() {
                            Some(back_buff) => {
                                let bytes = f32::to_le_bytes(back_buff);
                                channel.copy_from_slice(&bytes);
                            }
                            None => channel.copy_from_slice(&f32::to_le_bytes(0.0)),
                        },
                        None => channel.copy_from_slice(&f32::to_le_bytes(0.0)),
                    }
                }
            }

            let size = (stride * total_size) as u32;

            let chunk = pw_data.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = stride as i32;
            *chunk.size_mut() = size;
        }
    }
}
