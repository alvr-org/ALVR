use alvr_common::{data::*, logging::*, sockets::StreamReceiver, *};
use jni::{objects::GlobalRef, objects::ReleaseMode, JavaVM};
use std::{ptr, sync::Arc};

const NAL_TYPE_SPS: u8 = 7;
const H265_NAL_TYPE_VPS: u8 = 32;

// This function must not be async: it stores env, that is !Send. So this function will not cross
// await points and the execution is not sent between threads
pub fn push_to_decoder(
    java_vm: Arc<JavaVM>,
    activity_ref: Arc<GlobalRef>,
    data: &[u8],
    frame_index: u64,
) -> StrResult {
    let env = trace_err!(java_vm.attach_current_thread())?;

    let jnal = trace_err!(env.call_method(
        &*activity_ref,
        "getNALBuffer",
        "(I)Lcom/polygraphene/alvr/NAL;",
        &[(data.len() as i32).into()]
    ))?;

    let jnal = trace_err!(jnal.l())?;

    trace_err!(env.set_field(jnal, "length", "I", (data.len() as i32).into()))?;
    trace_err!(env.set_field(jnal, "frameIndex", "J", (frame_index as i64).into(),))?;

    let jbuf = trace_err!(trace_err!(env.get_field(jnal, "buf", "[B"))?.l())?;
    let (byte_arr_ptr, _) = trace_err!(env.get_byte_array_elements(*jbuf))?;
    unsafe { ptr::copy_nonoverlapping(data.as_ptr() as _, byte_arr_ptr, data.len()) };
    trace_err!(env.release_byte_array_elements(
        *jbuf,
        unsafe { &mut *byte_arr_ptr },
        ReleaseMode::CopyBack
    ))?;

    trace_err!(env.call_method(
        &*activity_ref,
        "pushNAL",
        "(Lcom/polygraphene/alvr/NAL;)V",
        &[jnal.into()],
    ))?;

    Ok(())
}

pub async fn receive_and_process_frames_loop(
    java_vm: Arc<JavaVM>,
    activity_ref: Arc<GlobalRef>,
    mut packet_receiver: StreamReceiver<VideoPacket>,
    codec: CodecType,
) -> StrResult {
    loop {
        let packet = match packet_receiver.recv().await {
            Ok(packet) => packet,
            Err(e) => {
                warn!("Error while listening for video packet: {}", e);
                continue;
            }
        };

        let nal_type = match codec {
            CodecType::H264 => packet.buffer[4] & 0x1F,
            CodecType::Hevc => packet.buffer[4] & 0x3F,
        };

        if (matches!(codec, CodecType::H264) && nal_type == NAL_TYPE_SPS)
            || (matches!(codec, CodecType::Hevc) && nal_type == H265_NAL_TYPE_VPS)
        {
            let header_nal_count = match codec {
                CodecType::H264 => 2, // SPS+PPS
                CodecType::Hevc => 3, // VPS+SPS+PPS
            };
            let maybe_begin_data_index = packet
                .buffer
                .windows(4)
                .enumerate()
                .filter_map(|(i, window)| {
                    if window == b"\x00\x00\x00\x01" {
                        Some(i)
                    } else {
                        None
                    }
                })
                .nth(header_nal_count + 1);

            if let Some(begin_data_index) = maybe_begin_data_index {
                push_to_decoder(
                    java_vm.clone(),
                    activity_ref.clone(),
                    &packet.buffer[..begin_data_index],
                    packet.tracking_index,
                )?;
                push_to_decoder(
                    java_vm.clone(),
                    activity_ref.clone(),
                    &packet.buffer[begin_data_index..],
                    packet.tracking_index,
                )?;
            }
        } else {
            push_to_decoder(
                java_vm.clone(),
                activity_ref.clone(),
                &packet.buffer,
                packet.tracking_index,
            )?;
        }
    }
}
