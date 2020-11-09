#![allow(non_upper_case_globals, non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use jni::{
    objects::*,
    sys::jintArray,
    sys::{jfloatArray, jobjectArray, jstring},
    *,
};

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_LatencyCollector_DecoderInput(
    _: JNIEnv,
    _: JObject,
    frame_index: i64,
) {
    decoderInput(frame_index);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_LatencyCollector_DecoderOutput(
    _: JNIEnv,
    _: JObject,
    frame_index: i64,
) {
    decoderOutput(frame_index);
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_initializeNative(
    env: JNIEnv,
    activity: JObject,
    asset_manager: JObject,
) {
    initializeNative(
        env.get_native_interface() as _,
        *activity as _,
        *asset_manager as _,
    )
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_destroyNative(
    env: JNIEnv,
    _: JObject,
) {
    destroyNative(env.get_native_interface() as _)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_getLoadingTextureNative(
    _: JNIEnv,
    _: JObject,
) -> i32 {
    getLoadingTextureNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_getSurfaceTextureIDNative(
    _: JNIEnv,
    _: JObject,
) -> i32 {
    getSurfaceTextureIDNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_getWebViewSurfaceTextureNative(
    _: JNIEnv,
    _: JObject,
) -> i32 {
    getWebViewSurfaceTextureNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_renderNative(
    _: JNIEnv,
    _: JObject,
    rendered_frame_index: i64,
) {
    renderNative(rendered_frame_index)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_renderLoadingNative(
    _: JNIEnv,
    _: JObject,
) {
    renderLoadingNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_sendTrackingInfoNative(
    env: JNIEnv,
    _: JObject,
    udp_receiver_thread: JObject,
) {
    sendTrackingInfoNative(env.get_native_interface() as _, *udp_receiver_thread as _)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_sendGuardianInfoNative(
    env: JNIEnv,
    _: JObject,
    udp_receiver_thread: JObject,
) {
    sendGuardianInfoNative(env.get_native_interface() as _, *udp_receiver_thread as _)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_sendMicDataNative(
    env: JNIEnv,
    _: JObject,
    udp_receiver_thread: JObject,
) {
    sendMicDataNative(env.get_native_interface() as _, *udp_receiver_thread as _)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onResumeNative(
    env: JNIEnv,
    _: JObject,
    surface: JObject,
) {
    onResumeNative(env.get_native_interface() as _, *surface as _)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onStreamStartNative(
    _: JNIEnv,
    _: JObject,
    width: i32,
    height: i32,
    refresh_rate: i32,
    stream_mic: u8,
    foveation_mode: i32,
    foveation_strength: f32,
    foveation_shape: f32,
    foveation_vertical_offset: f32,
) {
    onStreamStartNative(
        width,
        height,
        refresh_rate,
        stream_mic,
        foveation_mode,
        foveation_strength,
        foveation_shape,
        foveation_vertical_offset,
    )
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onPauseNative(
    _: JNIEnv,
    _: JObject,
) {
    onPauseNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_isVrModeNative(
    _: JNIEnv,
    _: JObject,
) -> u8 {
    isVrModeNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_getDeviceDescriptorNative(
    env: JNIEnv,
    _: JObject,
    device_descriptor: JObject,
) {
    getDeviceDescriptorNative(env.get_native_interface() as _, *device_descriptor as _)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onHapticsFeedbackNative(
    _: JNIEnv,
    _: JObject,
    start_time: i64,
    amplitude: f32,
    duration: f32,
    frequency: f32,
    hand: u8,
) {
    onHapticsFeedbackNative(start_time, amplitude, duration, frequency, hand)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onGuardianSyncAckNative(
    _: JNIEnv,
    _: JObject,
    timestamp: i64,
) {
    onGuardianSyncAckNative(timestamp)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_OvrActivity_onGuardianSegmentAckNative(
    _: JNIEnv,
    _: JObject,
    timestamp: i64,
    segment_index: i32,
) {
    onGuardianSegmentAckNative(timestamp, segment_index)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_ServerConnection_initializeSocket(
    env: JNIEnv,
    instance: JObject,
    hello_port: i32,
    port: i32,
    device_name: JString,
    broadcast_addr_list: jobjectArray,
    refresh_rates: jintArray,
    render_width: i32,
    render_height: i32,
) {
    initializeSocket(
        env.get_native_interface() as _,
        *instance as _,
        hello_port,
        port,
        **device_name as _,
        broadcast_addr_list as _,
        refresh_rates as _,
        render_width,
        render_height,
    )
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_ServerConnection_closeSocket(
    _: JNIEnv,
    _: JObject,
) {
    closeSocket()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_ServerConnection_runLoop(
    env: JNIEnv,
    instance: JObject,
) {
    runLoop(env.get_native_interface() as _, *instance as _)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_ServerConnection_interruptNative(
    _: JNIEnv,
    _: JObject,
) {
    interruptNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_ServerConnection_isConnectedNative(
    _: JNIEnv,
    _: JObject,
) -> u8 {
    isConnectedNative()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_ServerConnection_getServerAddress(
    env: JNIEnv,
    _: JObject,
) -> jstring {
    getServerAddress(env.get_native_interface() as _) as _
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_ServerConnection_getServerPort(
    _: JNIEnv,
    _: JObject,
) -> i32 {
    getServerPort()
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_ServerConnection_sendNative(
    _: JNIEnv,
    _: JObject,
    native_buffer: i64,
    buffer_length: i32,
) {
    sendNative(native_buffer, buffer_length)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_ServerConnection_setSinkPreparedNative(
    _: JNIEnv,
    _: JObject,
    prepared: u8,
) {
    setSinkPreparedNative(prepared)
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_polygraphene_alvr_Utils_setFrameLogEnabled(
    _: JNIEnv,
    _: JObject,
    debug_flags: i64,
) {
    setFrameLogEnabled(debug_flags)
}
