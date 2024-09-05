use alvr_common::{anyhow::Result, debug, error, parking_lot::Mutex, ConnectionError};
use alvr_session::AudioBufferingConfig;
use alvr_sockets::{StreamReceiver, StreamSender};
use pipewire::{
    self as pw,
    spa::{
        self,
        param::audio::{AudioFormat, AudioInfoRaw},
        pod::{self, serialize::PodSerializer, Pod},
    },
    stream::{StreamFlags, StreamListener, StreamState},
};
use std::{cmp, collections::VecDeque, sync::Arc, thread, time::Duration};
struct Terminate;

pub fn play_microphone_loop_pipewire(
    running: impl Fn() -> bool,
    channels_count: u16,
    sample_rate: u32,
    config: AudioBufferingConfig,
    receiver: &mut StreamReceiver<()>,
) -> Result<()> {
    let batch_frames_count = sample_rate as usize * config.batch_ms as usize / 1000;
    let average_buffer_frames_count =
        sample_rate as usize * config.average_buffering_ms as usize / 1000;

    let sample_buffer: Arc<
        alvr_common::parking_lot::lock_api::Mutex<
            alvr_common::parking_lot::RawMutex,
            VecDeque<f32>,
        >,
    > = Arc::new(Mutex::new(VecDeque::new()));

    let (pw_sender, pw_receiver) = pw::channel::channel();
    let pw_loop_buffer_arc = Arc::clone(&sample_buffer);
    let receive_samples_buffer_arc = Arc::clone(&sample_buffer);
    let pw_stream_state = Arc::new(Mutex::new(StreamState::Unconnected));
    let pw_stream_state_arc = Arc::clone(&pw_stream_state);
    let thread = thread::spawn(move || {
        match pw_microphone_loop(
            pw_stream_state,
            sample_rate,
            channels_count,
            pw_receiver,
            pw_loop_buffer_arc,
        ) {
            Ok(_) => {
                debug!("Pipewire loop exiting");
            }
            Err(e) => error!("Pipewire error: {}", e.to_string()),
        }
    });

    while running() {
        let stream_audio = {
            || {
                if let Some(stream_state) = pw_stream_state_arc.try_lock() {
                    *stream_state == StreamState::Streaming && running()
                } else {
                    false
                }
            }
        };
        let receive_samples_buffer_arc = Arc::clone(&receive_samples_buffer_arc);
        crate::receive_samples_loop(
            stream_audio,
            receiver,
            receive_samples_buffer_arc,
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
        };
    }

    if pw_sender.send(Terminate).is_err() {
        error!(
            "Couldn't send pipewire termination signal, deinitializing forcefully.
            Restart VR app to reinitialize pipewire."
        );
        unsafe { pw::deinit() };
    }

    match thread.join() {
        Ok(_) => debug!("Pipewire thread joined"),
        Err(_) => {
            error!("Couldn't wait for pipewire thread to finish");
        }
    }
    Ok(())
}

fn pw_microphone_loop(
    pw_stream_state: Arc<Mutex<StreamState>>,
    sample_rate: u32,
    channels_count: u16,
    pw_receiver: pw::channel::Receiver<Terminate>,
    sample_buffer: Arc<Mutex<VecDeque<f32>>>,
) -> Result<(), pw::Error> {
    debug!("Starting microphone pw-thread");
    let mainloop = pw::main_loop::MainLoop::new(None)?;

    let _receiver = pw_receiver.attach(mainloop.as_ref(), {
        let mainloop = mainloop.clone();
        move |_| mainloop.quit()
    });

    let context = pw::context::Context::new(&mainloop)?;
    let core = context.connect(None)?;

    let stream = pw::stream::Stream::new(
        &core,
        "alvr-mic",
        pw::properties::properties! {
            *pw::keys::NODE_NAME => "ALVR Microphone",
            *pw::keys::MEDIA_NAME => "alvr-mic",
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Playback",
            *pw::keys::MEDIA_CLASS => "Audio/Source",
            *pw::keys::MEDIA_ROLE => "Communication",
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
        .process(move |stream, _| match stream.dequeue_buffer() {
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
        })
        .register()?;

    let mut audio_info = AudioInfoRaw::new();
    audio_info.set_format(AudioFormat::F32LE);
    audio_info.set_rate(sample_rate);
    audio_info.set_channels(channels_count.into());

    let values: Vec<u8> = PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pod::Value::Object(pod::Object {
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
        spa::utils::Direction::Output,
        None,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
        &mut params,
    )?;
    debug!("Prepared microphone pw-thread");

    mainloop.run();
    Ok(())
}

pub fn record_audio_blocking_pipewire(
    is_running: Arc<dyn Fn() -> bool + Send + Sync>,
    sender: StreamSender<()>,
    channels_count: u16,
    sample_rate: u32,
) -> Result<(), ()> {
    let (pw_sender, pw_receiver) = pw::channel::channel();
    let is_running_clone_for_pw_terminate: Arc<dyn Fn() -> bool + Send + Sync> =
        Arc::clone(&is_running);
    thread::spawn(move || {
        while is_running_clone_for_pw_terminate() {
            thread::sleep(Duration::from_millis(500));
        }
        if pw_sender.send(Terminate).is_err() {
            error!(
                "Couldn't send pipewire termination signal, deinitializing forcefully.
                Restart VR app to reinitialize pipewire."
            );
            unsafe { pw::deinit() };
        }
    });
    let is_running_clone_for_pw = Arc::clone(&is_running);
    match pw_audio_loop(
        sample_rate,
        channels_count,
        pw_receiver,
        sender,
        is_running_clone_for_pw,
    ) {
        Ok(_) => {
            debug!("Pipewire loop exiting");
        }
        Err(e) => error!("Pipewire error: {}", e.to_string()),
    }
    Ok(())
}

fn pw_audio_loop(
    sample_rate: u32,
    channels_count: u16,
    pw_receiver: pw::channel::Receiver<Terminate>,
    mut sender: StreamSender<()>,
    is_running: Arc<dyn Fn() -> bool + Send + Sync>,
) -> Result<(), pw::Error> {
    debug!("Starting audio pw-thread");

    let mainloop = pw::main_loop::MainLoop::new(None)?;

    let _receiver = pw_receiver.attach(mainloop.as_ref(), {
        let mainloop = mainloop.clone();
        move |_| mainloop.quit()
    });

    let context = pw::context::Context::new(&mainloop)?;
    let core = context.connect(None)?;

    let stream = pw::stream::Stream::new(
        &core,
        "alvr-audio",
        pw::properties::properties! {
            *pw::keys::NODE_NAME => "ALVR Audio",
            *pw::keys::MEDIA_NAME => "alvr-audio",
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_CLASS => "Audio/Sink",
            *pw::keys::MEDIA_ROLE => "Game",
        },
    )?;

    let chan_size = std::mem::size_of::<i16>();

    let _listener: StreamListener<i16> = stream
        .add_local_listener()
        .process(move |stream, _| match stream.dequeue_buffer() {
            None => {
                // Nothing is connected to stream, continue
            }
            Some(mut pw_buffer) => {
                let datas = pw_buffer.datas_mut();
                if datas.is_empty() {
                    return;
                }

                let pw_data = &mut datas[0];
                let stride = chan_size * channels_count as usize;
                let n_frames = (pw_data.chunk().size() / stride as u32) as usize;
                let mut final_buffer: Vec<u8> = Vec::with_capacity(n_frames);
                if let Some(slice) = pw_data.data() {
                    for n_frame in 0..n_frames {
                        for n_channel in 0..channels_count {
                            let start = n_frame * stride + (n_channel as usize * chan_size);
                            let end = start + chan_size;
                            let channel = &mut slice[start..end];
                            let slice =
                                i16::from_ne_bytes(channel.try_into().unwrap()).to_ne_bytes();
                            final_buffer.extend(slice.iter());
                        }
                    }
                }
                if !final_buffer.is_empty() && is_running() {
                    let mut buffer = sender.get_buffer(&()).unwrap();
                    buffer
                        .get_range_mut(0, final_buffer.len())
                        .copy_from_slice(&final_buffer);
                    sender.send(buffer).ok();
                }
            }
        })
        .register()?;

    let mut audio_info = AudioInfoRaw::new();
    audio_info.set_format(AudioFormat::S16LE);
    audio_info.set_rate(sample_rate);
    audio_info.set_channels(channels_count.into());

    let values: Vec<u8> = PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pod::Value::Object(pod::Object {
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
        spa::utils::Direction::Input,
        None,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
        &mut params,
    )?;
    debug!("Prepared audio pw-thread");

    mainloop.run();
    Ok(())
}
