use std::ffi::{c_char, c_void, CStr};
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, Notify};
use tracing::{debug, error};

use super::client::Message;
use crate::common::Buffer;

fn call<T>(interface: *const c_void, functor: impl FnOnce(&T) + std::panic::UnwindSafe) {
    let res = std::panic::catch_unwind(|| {
        // The code is checking if the `interface` pointer is not null and if it can be
        // safely cast to a reference of type `Interface`. If both conditions
        // are true, it calls the `functor` function with the `interface` as an
        // argument. This is a way to safely access the `Interface` object from
        // the raw `c_void` pointer passed to the callback functions.
        if let Some(interface) = unsafe { interface.cast::<T>().as_ref() } {
            functor(interface);
        }
    });

    if let Err(err) = res {
        error!(?err, "call to interface panicked");
    }
}

#[no_mangle]
extern "C" fn on_connection_success(
    interface: *const c_void,
    service_id_1: Buffer,
    service_id_2: Buffer,
    service_id_3: Buffer,
) {
    call::<TunnelInterface>(interface, |interface| {
        interface.connection_success.as_ref()(service_id_1, service_id_2, service_id_3);
    });
}

#[no_mangle]
extern "C" fn on_connection_failure(interface: *const c_void, error_code: i32) {
    call::<TunnelInterface>(interface, |interface| {
        interface.connection_failure.as_ref()(error_code);
    });
}

#[no_mangle]
extern "C" fn on_connection_shutdown(interface: *const c_void) {
    call::<TunnelInterface>(interface, |interface| {
        interface.connection_shutdown.as_ref()();
    });
}

#[no_mangle]
extern "C" fn on_connection_reset(
    interface: *const c_void,
    error_code: i32,
    connection_id: u32,
    service_id: Buffer,
) {
    call::<TunnelInterface>(interface, |interface| {
        interface.connection_reset.as_ref()(error_code, connection_id, service_id);
    });
}

#[no_mangle]
extern "C" fn on_session_reset(interface: *const c_void) {
    call::<TunnelInterface>(interface, |interface| {
        interface.session_reset.as_ref()();
    });
}

#[no_mangle]
extern "C" fn on_send_message_complete(
    interface: *const c_void,
    error_code: i32,
    message_type: Buffer,
) {
    call::<TunnelInterface>(interface, |interface| {
        interface.send_message_complete.as_ref()(error_code, message_type);
    });
}

#[no_mangle]
extern "C" fn on_message_received(
    interface: *const c_void,
    connection_id: u32,
    payload: Buffer,
    service_id: Buffer,
) {
    call::<TunnelInterface>(interface, |interface| {
        interface.message_received.as_ref()(connection_id, payload, service_id);
    });
}

#[no_mangle]
extern "C" fn on_stream_started(
    interface: *const c_void,
    error_code: i32,
    connection_id: u32,
    service_id: Buffer,
) {
    call::<TunnelInterface>(interface, |interface| {
        interface.stream_started.as_ref()(error_code, connection_id, service_id);
    });
}

#[no_mangle]
extern "C" fn on_stream_stopped(interface: *const c_void, service_id: Buffer) {
    call::<TunnelInterface>(interface, |interface| {
        interface.stream_stopped.as_ref()(service_id);
    });
}

#[no_mangle]
extern "C" fn on_subscribe_complete(interface: *const c_void, error_code: i32) {
    call::<ClientInterface>(interface, |interface| {
        interface.subscribe_complete.as_ref()(error_code);
    });
}

#[no_mangle]
extern "C" fn on_subscribe_tunnel(
    interface: *const c_void,
    access_token: *const c_char,
    region: *const c_char,
    client_mode: *const c_char,
) {
    call::<ClientInterface>(interface, |interface| {
        interface.subscribe_tunnel.as_ref()(access_token, region, client_mode);
    });
}

pub(super) fn create_connection_success_callback() -> impl Fn(Buffer, Buffer, Buffer) {
    move |service_id_1, service_id_2, service_id_3| {
        if let Ok(service_id_1) = String::try_from(service_id_1) {
            debug!(%service_id_1);
        }

        if let Ok(service_id_2) = String::try_from(service_id_2) {
            debug!(%service_id_2);
        }

        if let Ok(service_id_3) = String::try_from(service_id_3) {
            debug!(%service_id_3);
        }
    }
}

pub(super) fn create_session_reset_callback(session_reset: Arc<Notify>) -> impl Fn() {
    move || {
        session_reset.notify_waiters();
    }
}

pub(super) fn create_connection_failure_callback(
    connection_failure_tx: broadcast::Sender<i32>,
) -> impl Fn(i32) {
    move |error_code| {
        let _ = connection_failure_tx.send(error_code);
    }
}

pub(super) fn create_connection_shutdown_callback(connection_shutdown: Arc<Notify>) -> impl Fn() {
    move || {
        debug!("shutdown");
        connection_shutdown.notify_waiters();
    }
}

pub(super) fn create_connection_reset_callback(
    connection_reset_tx: broadcast::Sender<(i32, u32, Option<String>)>,
) -> impl Fn(i32, u32, Buffer) {
    move |error_code, connection_id, service_id| {
        let _ = connection_reset_tx.send((
            error_code,
            connection_id,
            String::try_from(service_id).ok(),
        ));
    }
}

pub(super) fn create_send_message_complete_callback() -> impl Fn(i32, Buffer) {
    move |_error_code, _message_type| {}
}

pub(super) fn create_message_received_callback(
    message_receive_tx: mpsc::Sender<Message>,
) -> impl Fn(u32, Buffer, Buffer) {
    move |connection_id, payload, _service_id| {
        if payload.is_empty() {
            error!("payload is empty");
            return;
        }

        let _ = message_receive_tx.try_send(Message {
            _connection_id: connection_id,
            payload: payload.into(),
        });
    }
}

pub(super) fn create_stream_started_callback(
    stream_start_tx: mpsc::Sender<u32>,
) -> impl Fn(i32, u32, Buffer) {
    move |error_code, connection_id, service_id| {
        if error_code == 0 {
            let _ = stream_start_tx.try_send(connection_id);
        }

        debug!(%error_code, %connection_id, service_id = ?String::try_from(service_id), "stream started");
    }
}

pub(super) fn create_stream_stopped_callback(stream_stop: Arc<Notify>) -> impl Fn(Buffer) {
    move |_service_id| {
        stream_stop.notify_waiters();
    }
}

pub(super) fn create_subscribe_complete_callback(
    client_created_tx: mpsc::Sender<i32>,
) -> impl Fn(i32) {
    move |error_code| {
        let _ = client_created_tx.try_send(error_code);
    }
}

pub(super) fn create_subscribe_tunnel_callback(
    tunnel_created_tx: mpsc::Sender<Credentials>,
) -> impl Fn(*const c_char, *const c_char, *const c_char) {
    move |access_token: *const c_char, region: *const c_char, client_mode: *const c_char| {
        let _ = tunnel_created_tx.try_send(Credentials {
            access_token: unsafe { CStr::from_ptr(access_token) }
                .to_string_lossy()
                .to_string(),
            region: unsafe { CStr::from_ptr(region) }
                .to_string_lossy()
                .to_string(),
            _client_mode: unsafe { CStr::from_ptr(client_mode) }
                .to_string_lossy()
                .to_string(),
        });
    }
}

#[derive(Debug)]
pub(super) struct Credentials {
    pub(super) access_token: String,
    pub(super) region: String,
    pub(super) _client_mode: String,
}

pub(super) struct ClientInterface {
    pub(super) subscribe_complete: Box<dyn Fn(i32)>,
    pub(super) subscribe_tunnel: Box<dyn Fn(*const c_char, *const c_char, *const c_char)>,
}

pub(super) struct TunnelInterface {
    pub(super) connection_success: Box<dyn Fn(Buffer, Buffer, Buffer)>,
    pub(super) connection_failure: Box<dyn Fn(i32)>,
    pub(super) connection_shutdown: Box<dyn Fn()>,
    pub(super) connection_reset: Box<dyn Fn(i32, u32, Buffer)>,
    pub(super) session_reset: Box<dyn Fn()>,
    pub(super) send_message_complete: Box<dyn Fn(i32, Buffer)>,
    pub(super) message_received: Box<dyn Fn(u32, Buffer, Buffer)>,
    pub(super) stream_started: Box<dyn Fn(i32, u32, Buffer)>,
    pub(super) stream_stopped: Box<dyn Fn(Buffer)>,
}

unsafe impl Send for ClientInterface {}
unsafe impl Sync for ClientInterface {}

unsafe impl Send for TunnelInterface {}
unsafe impl Sync for TunnelInterface {}
