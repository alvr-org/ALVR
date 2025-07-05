#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
use alvr_common::{anyhow::Result, debug, error};
use std::{io, sync::Arc, thread, time::Duration};

use alvr_sockets::StreamSender;
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

use crate::linux::Terminate;

pub fn record_audio_blocking(
    is_running: Arc<dyn Fn() -> bool + Send + Sync>,
    sender: StreamSender<()>,
    channels_count: u16,
    sample_rate: u32,
) -> Result<()> {
    let (pw_sender, pw_receiver) = pipewire::channel::channel();
    let is_running_for_pipewire = Arc::clone(&is_running);

    let thread_handle = audio_loop_thread(pw_sender, is_running_for_pipewire);
    let is_running_for_pipewire = Arc::clone(&is_running);
    match pipewire_main_loop(
        sample_rate,
        channels_count,
        pw_receiver,
        sender,
        is_running_for_pipewire,
    ) {
        Ok(_) => {
            debug!("Pipewire audio loop exiting");
        }
        Err(e) => {
            error!("Unhandled pipewire microphone device error, please report it to GitHub: {e}");
        }
    }
    match thread_handle.join() {
        Ok(()) => debug!("Pipewire audio thread joined"),
        Err(_) => {
            error!("Couldn't wait for pipewire audio thread to finish.");
        }
    }
    Ok(())
}
fn audio_loop_thread(
    pw_sender: pipewire::channel::Sender<Terminate>,
    is_running_for_pipewire: Arc<dyn Fn() -> bool + Send + Sync + 'static>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while is_running_for_pipewire() {
            thread::sleep(Duration::from_millis(500));
        }
        debug!("Pipewire audio loop thread terminating");
        if pw_sender.send(Terminate).is_err() {
            error!(
                "Couldn't send pipewire termination signal, deinitializing forcefully.
                Restart VR app to reinitialize audio device."
            );
            unsafe { pipewire::deinit() };
        }
    })
}

fn pipewire_main_loop(
    sample_rate: u32,
    channels_count: u16,
    pw_receiver: Receiver<Terminate>,
    mut sender: StreamSender<()>,
    is_running: Arc<dyn Fn() -> bool + Send + Sync>,
) -> Result<(), pipewire::Error> {
    debug!("Starting audio pw-thread");

    let mainloop = MainLoop::new(None)?;

    let _receiver = pw_receiver.attach(mainloop.as_ref(), {
        let mainloop = mainloop.clone();
        move |_| mainloop.quit()
    });

    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;

    let stream = Stream::new(
        &core,
        "alvr-audio",
        properties::properties! {
            *keys::NODE_NAME => "ALVR Audio",
            *keys::MEDIA_NAME => "alvr-audio",
            *keys::MEDIA_TYPE => "Audio",
            *keys::MEDIA_CATEGORY => "Capture",
            *keys::MEDIA_CLASS => "Audio/Sink",
            *keys::MEDIA_ROLE => "Game",
        },
    )?;

    let chan_size = std::mem::size_of::<i16>();
    let _listener: StreamListener<i16> = stream
        .add_local_listener()
        .process(move |stream, _| {
            match stream.dequeue_buffer() {
                None => {
                    // Nothing is connected to stream, continue
                }
                Some(pw_buffer) => {
                    if let Some(out_buffer) =
                        pipewire_buffer_process(channels_count, chan_size, pw_buffer)
                        && !out_buffer.is_empty()
                        && is_running()
                    {
                        let mut buffer = sender.get_buffer(&()).unwrap();
                        buffer
                            .get_range_mut(0, out_buffer.len())
                            .copy_from_slice(&out_buffer);
                        sender.send(buffer).ok();
                    }
                }
            }
        })
        .register()?;

    let mut audio_info = AudioInfoRaw::new();
    audio_info.set_format(AudioFormat::S16LE);
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
        Direction::Input,
        None,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
        &mut params,
    )?;
    debug!("Prepared audio pw-thread");

    mainloop.run();
    Ok(())
}

fn pipewire_buffer_process(
    channels_count: u16,
    chan_size: usize,
    mut pw_buffer: pipewire::buffer::Buffer<'_>,
) -> Option<Vec<u8>> {
    let datas = pw_buffer.datas_mut();
    if datas.is_empty() {
        return None;
    }
    let pw_data = &mut datas[0];
    let stride = chan_size * channels_count as usize;
    let n_frames = (pw_data.chunk().size() / stride as u32) as usize;
    let mut out_buffer: Vec<u8> = Vec::with_capacity(n_frames);
    if let Some(slice) = pw_data.data() {
        for n_frame in 0..n_frames {
            for n_channel in 0..channels_count {
                let start = n_frame * stride + (n_channel as usize * chan_size);
                let end = start + chan_size;
                let channel = &mut slice[start..end];
                let slice = i16::from_ne_bytes(channel.try_into().unwrap()).to_ne_bytes();
                out_buffer.extend_from_slice(&slice);
            }
        }
    }
    Some(out_buffer)
}
