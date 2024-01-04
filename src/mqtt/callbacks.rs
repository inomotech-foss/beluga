//! Defines several callbacks that could be called from C/C++ side

use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::os::raw::c_void;
use std::sync::Arc;

use crossbeam::queue::SegQueue;
use log::{as_debug, as_display, debug, error};
use parking_lot::FairMutex;
use tokio::sync::oneshot::Sender;

use super::client::Subscriber;
use super::{ClientStatus, Message};
use crate::common::{AwsMqttConnectReturnCode, AwsMqttError, Buffer, Qos};

fn call(interface: *const c_void, functor: impl FnOnce(&Interface) + std::panic::UnwindSafe) {
    let res = std::panic::catch_unwind(|| {
        // The code is checking if the `interface` pointer is not null and if it can be
        // safely cast to a reference of type `Interface`. If both conditions
        // are true, it calls the `functor` function with the `interface` as an
        // argument. This is a way to safely access the `Interface` object from
        // the raw `c_void` pointer passed to the callback functions.
        if let Some(interface) = unsafe { interface.cast::<Interface>().as_ref() } {
            functor(interface);
        }
    });

    if let Err(err) = res {
        error!(error = as_debug!(err); "call to interface panicked");
    }
}

#[no_mangle]
extern "C" fn on_completed(
    interface: *const c_void,
    error_code: i32,
    return_code: AwsMqttConnectReturnCode,
    session_present: bool,
) {
    call(interface, |interface| {
        interface.completed.as_ref()(error_code, return_code, session_present);
    });
}

#[no_mangle]
extern "C" fn on_closed(interface: *const c_void) {
    call(interface, |interface| {
        interface.closed.as_ref()();
    });
}

#[no_mangle]
extern "C" fn on_interrupted(interface: *const c_void, error: i32) {
    call(interface, |interface| {
        interface.interrupted.as_ref()(error);
    });
}

#[no_mangle]
extern "C" fn on_resumed(
    interface: *const c_void,
    return_code: AwsMqttConnectReturnCode,
    session_present: bool,
) {
    call(interface, |interface| {
        interface.resumed.as_ref()(return_code, session_present);
    });
}

#[no_mangle]
extern "C" fn on_message(
    interface: *const c_void,
    topic: *const c_char,
    data: Buffer,
    dup: bool,
    qos: Qos,
    retain: bool,
) {
    call(interface, |interface| {
        interface.message.as_ref()(topic, data, dup, qos, retain);
    });
}

#[no_mangle]
extern "C" fn on_sub_ack(
    interface: *const c_void,
    packet_id: u16,
    topic: *const c_char,
    qos: Qos,
    error_code: i32,
) {
    call(interface, |interface| {
        interface.sub_ack.as_ref()(packet_id, topic, qos, error_code);
    });
}

#[no_mangle]
extern "C" fn on_publish(interface: *const c_void, packet_id: u16, error_code: i32) {
    call(interface, |interface| {
        interface.publish.as_ref()(packet_id, error_code);
    });
}

#[no_mangle]
extern "C" fn on_unsubscribe(interface: *const c_void, packet_id: u16, error_code: i32) {
    call(interface, |interface| {
        interface.unsubscribe.as_ref()(packet_id, error_code);
    });
}

pub(super) fn create_completed_callback(
    status: Arc<FairMutex<ClientStatus>>,
    notify_future: Arc<FairMutex<Option<Sender<ClientStatus>>>>,
) -> impl Fn(i32, AwsMqttConnectReturnCode, bool) {
    move |error_code, return_code, session_present| {
        if let Ok(error) = AwsMqttError::try_from(error_code) {
            debug!(return_code = as_display!(return_code), session_present = session_present, error = as_display!(error); "on completed triggered");
        } else {
            debug!(return_code = as_display!(return_code), session_present = session_present; "on completed triggered");
        }

        if error_code != 0 && !matches!(return_code, AwsMqttConnectReturnCode::Accepted) {
            *status.lock() = ClientStatus::Closed;
            if let Some(notify) = notify_future.lock().take() {
                let _ = notify.send(ClientStatus::Closed);
            };
        } else {
            *status.lock() = ClientStatus::Connected;
            if let Some(notify) = notify_future.lock().take() {
                let _ = notify.send(ClientStatus::Connected);
            };
        }
    }
}

pub(super) fn create_closed_callback(status: Arc<FairMutex<ClientStatus>>) -> impl Fn() {
    move || {
        debug!("on closed triggered");
        *status.lock() = ClientStatus::Closed;
    }
}

pub(super) fn create_interrupted_callback(status: Arc<FairMutex<ClientStatus>>) -> impl Fn(i32) {
    move |error_code| {
        if let Ok(error) = AwsMqttError::try_from(error_code) {
            debug!(error = as_display!(error); "on interrupted triggered");
        } else {
            debug!(error_code = as_display!(error_code); "on interrupted triggered");
        }
        *status.lock() = ClientStatus::Interrupted;
    }
}

pub(super) fn create_resumed_callback(
    status: Arc<FairMutex<ClientStatus>>,
) -> impl Fn(AwsMqttConnectReturnCode, bool) {
    move |return_code, session_present| {
        debug!(return_code = as_display!(return_code), session_present = session_present; "on resumed triggered");
        if let AwsMqttConnectReturnCode::Accepted = return_code {
            *status.lock() = ClientStatus::Connected;
        }
    }
}

pub(super) fn create_message_callback(
    subscribers: Arc<SegQueue<Subscriber>>,
) -> impl Fn(*const c_char, Buffer, bool, Qos, bool) {
    move |topic, data, dup, qos, retain| {
        let topic = unsafe { CStr::from_ptr(topic) }
            .to_string_lossy()
            .to_string();

        let msg = Message {
            topic: topic.clone(),
            data: data.into(),
            dup,
            qos,
            retain,
        };

        let queue_size = subscribers.len();

        // The code is iterating over the subscribers in a loop, checking if each
        // subscriber's topic matches the received message's topic. If there is
        // a match, the message is sent to that subscriber using the
        // `send_message` method. If there is no match, but the subscriber is not
        // closed, it is pushed back into the subscribers queue for future messages.
        // This loop ensures that the message is delivered to all subscribers
        // whose topics match the received message.
        for _ in 0..queue_size {
            if let Some(subscriber) = subscribers.pop() {
                if subscriber.contains(&topic) {
                    subscriber.send_message(msg.clone());
                } else if !subscriber.is_closed() {
                    subscribers.push(subscriber);
                }
            }
        }
    }
}

pub(super) fn create_sub_ack_callback() -> impl Fn(u16, *const c_char, Qos, i32) {
    move |_, _, _, _| {}
}

pub(super) fn create_notify_callback(
    notifiers: Arc<FairMutex<HashMap<u16, Sender<i32>>>>,
) -> impl Fn(u16, i32) {
    move |packet_id, error_code| {
        if let Some(notify_future) = notifiers.lock().remove(&packet_id) {
            let _ = notify_future.send(error_code);
        }
    }
}

pub(super) struct Interface {
    pub(super) completed: Box<dyn Fn(i32, AwsMqttConnectReturnCode, bool)>,
    pub(super) closed: Box<dyn Fn()>,
    pub(super) interrupted: Box<dyn Fn(i32)>,
    pub(super) resumed: Box<dyn Fn(AwsMqttConnectReturnCode, bool)>,
    pub(super) message: Box<dyn Fn(*const c_char, Buffer, bool, Qos, bool)>,
    pub(super) sub_ack: Box<dyn Fn(u16, *const c_char, Qos, i32)>,
    pub(super) publish: Box<dyn Fn(u16, i32)>,
    pub(super) unsubscribe: Box<dyn Fn(u16, i32)>,
}

unsafe impl Send for Interface {}
unsafe impl Sync for Interface {}
