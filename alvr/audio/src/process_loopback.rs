//! Game audio captured with Windows' process loopback: `ActivateAudioInterfaceAsync` on `VAD\Process_Loopback` returns everything the PC plays
//! except ALVR's own process tree, before the output device's volume, mute and format are applied, as plain PCM in a format we choose (48 kHz
//! stereo 16 bit). No virtual audio cable and no change of the default output device are needed. Requires Windows 10 version 2004 or newer.

use ::windows::{
    Win32::{
        Foundation::{CloseHandle, E_FAIL, WAIT_OBJECT_0},
        Media::Audio::{
            AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            AUDCLNT_STREAMFLAGS_LOOPBACK, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
            AUDIOCLIENT_ACTIVATION_PARAMS, AUDIOCLIENT_ACTIVATION_PARAMS_0,
            AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK, AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS,
            ActivateAudioInterfaceAsync, IActivateAudioInterfaceAsyncOperation,
            IActivateAudioInterfaceCompletionHandler,
            IActivateAudioInterfaceCompletionHandler_Impl, IAudioCaptureClient, IAudioClient,
            PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE, WAVEFORMATEX,
        },
        System::{
            Com::{
                BLOB, COINIT_MULTITHREADED, CoInitializeEx,
                StructuredStorage::{
                    PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
                },
            },
            Threading::{CreateEventW, WaitForSingleObject},
            Variant::VT_BLOB,
        },
    },
    core::{Interface, Ref, Result as WinResult, implement, w},
};
use alvr_common::anyhow::{Result, anyhow, bail};
use alvr_sockets::StreamSender;
use std::{
    mem::ManuallyDrop,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

/// The sample rate the captured audio is converted to, and so the one the client is told.
pub const PROCESS_LOOPBACK_SAMPLE_RATE: u32 = 48000;

type Activation = Arc<(Mutex<Option<WinResult<IAudioClient>>>, Condvar)>;

#[implement(IActivateAudioInterfaceCompletionHandler)]
struct ActivationHandler {
    activation: Activation,
}

impl IActivateAudioInterfaceCompletionHandler_Impl for ActivationHandler_Impl {
    fn ActivateCompleted(
        &self,
        operation: Ref<'_, IActivateAudioInterfaceAsyncOperation>,
    ) -> WinResult<()> {
        let client = (|| {
            let mut result = ::windows::core::HRESULT(0);
            let mut interface = None;
            unsafe {
                operation
                    .ok()?
                    .GetActivateResult(&mut result, &mut interface)?
            };
            result.ok()?;
            interface
                .ok_or_else(|| ::windows::core::Error::from(E_FAIL))?
                .cast::<IAudioClient>()
        })();

        let (slot, ready) = &*self.activation;
        *slot.lock().unwrap() = Some(client);
        ready.notify_all();

        Ok(())
    }
}

fn activate_excluding_this_process() -> Result<IAudioClient> {
    let params = AUDIOCLIENT_ACTIVATION_PARAMS {
        ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
        Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
            ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                TargetProcessId: std::process::id(),
                ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE,
            },
        },
    };

    // Never dropped: dropping a PROPVARIANT clears it, which would free the blob, and the blob lives on this stack.
    let property = ManuallyDrop::new(PROPVARIANT {
        Anonymous: PROPVARIANT_0 {
            Anonymous: ManuallyDrop::new(PROPVARIANT_0_0 {
                vt: VT_BLOB,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: PROPVARIANT_0_0_0 {
                    blob: BLOB {
                        cbSize: size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32,
                        pBlobData: &params as *const _ as *mut u8,
                    },
                },
            }),
        },
    });

    let activation: Activation = Arc::new((Mutex::new(None), Condvar::new()));
    let handler: IActivateAudioInterfaceCompletionHandler = ActivationHandler {
        activation: Arc::clone(&activation),
    }
    .into();

    let _operation = unsafe {
        ActivateAudioInterfaceAsync(
            w!("VAD\\Process_Loopback"),
            &IAudioClient::IID,
            Some(&*property),
            &handler,
        )?
    };

    let (slot, ready) = &*activation;
    let (mut slot, wait) = ready
        .wait_timeout_while(slot.lock().unwrap(), Duration::from_secs(10), |client| {
            client.is_none()
        })
        .unwrap();
    if wait.timed_out() {
        bail!("Process loopback activation timed out");
    }

    slot.take()
        .ok_or_else(|| anyhow!("Process loopback activation returned nothing"))?
        .map_err(|e| anyhow!("Process loopback activation failed: {e}"))
}

/// Whether the process loopback capture can be started on this Windows (it needs Windows 10 version 2004 or newer).
pub fn process_loopback_available() -> bool {
    // Ignored on purpose: the thread may already have initialized COM, with any apartment model.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().ok() };

    activate_excluding_this_process().is_ok()
}

/// Captures everything the PC plays except this process and sends it as 48 kHz stereo 16 bit PCM until `is_running` returns false.
pub fn record_process_loopback_blocking(
    is_running: Arc<dyn Fn() -> bool + Send + Sync>,
    mut sender: StreamSender<()>,
) -> Result<()> {
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().ok() };

    let client = activate_excluding_this_process()?;

    let format = WAVEFORMATEX {
        wFormatTag: 1, // WAVE_FORMAT_PCM
        nChannels: 2,
        nSamplesPerSec: PROCESS_LOOPBACK_SAMPLE_RATE,
        nAvgBytesPerSec: PROCESS_LOOPBACK_SAMPLE_RATE * 4,
        nBlockAlign: 4,
        wBitsPerSample: 16,
        cbSize: 0,
    };
    let flags = AUDCLNT_STREAMFLAGS_LOOPBACK
        | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
        | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
        | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
    unsafe {
        client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 1_000_000, 0, &format, None)?;
    }

    let event = unsafe { CreateEventW(None, false, false, None)? };
    unsafe { client.SetEventHandle(event)? };
    let capture: IAudioCaptureClient = unsafe { client.GetService()? };

    unsafe { client.Start()? };

    let mut silence = Vec::new();
    let mut result = Ok(());
    'capture: while is_running() {
        if unsafe { WaitForSingleObject(event, 100) } != WAIT_OBJECT_0 {
            continue;
        }

        loop {
            match unsafe { capture.GetNextPacketSize() } {
                Ok(0) => break,
                Ok(_) => (),
                Err(e) => {
                    result = Err(anyhow!("Process loopback capture failed: {e}"));
                    break 'capture;
                }
            }

            let (mut data, mut frames, mut buffer_flags) = (std::ptr::null_mut(), 0, 0);
            if let Err(e) =
                unsafe { capture.GetBuffer(&mut data, &mut frames, &mut buffer_flags, None, None) }
            {
                result = Err(anyhow!("Process loopback capture failed: {e}"));
                break 'capture;
            }

            let size = frames as usize * format.nBlockAlign as usize;
            let payload =
                if buffer_flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null() {
                    silence.resize(size, 0);
                    &silence[..]
                } else {
                    unsafe { std::slice::from_raw_parts(data, size) }
                };
            sender.send_header_with_payload(&(), payload).ok();

            unsafe { capture.ReleaseBuffer(frames).ok() };
        }
    }

    unsafe {
        client.Stop().ok();
        CloseHandle(event).ok();
    }

    result
}
