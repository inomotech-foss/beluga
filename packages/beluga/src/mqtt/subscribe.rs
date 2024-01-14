use alloc::boxed::Box;
use core::ffi::c_void;

use aws_c_common_sys::aws_byte_cursor;
use aws_c_mqtt_sys::{
    aws_mqtt_client_connection, aws_mqtt_client_publish_received_fn, aws_mqtt_qos,
    aws_mqtt_userdata_cleanup_fn,
};

use super::Qos;
use crate::ByteCursor;

pub struct MessageRef<'a> {
    pub topic: &'a str,
    pub payload: &'a [u8],
    pub dup: bool,
    pub qos: Qos,
    pub retain: bool,
}

pub struct SubscribeAck {
    pub granted_qos: Qos,
}

pub struct PublishCallback {
    pub on_publish: aws_mqtt_client_publish_received_fn,
    // userdata is already owned here!
    pub userdata: *mut c_void,
    pub cleanup_userdata: aws_mqtt_userdata_cleanup_fn,
}

impl PublishCallback {
    // # Safety
    //
    // It must be ensured that this is only called if the ownership hasn't been
    // transferred to the C side yet.
    pub unsafe fn cleanup(self) {
        if let Some(cleanup) = self.cleanup_userdata {
            unsafe { cleanup(self.userdata) };
        }
    }
}

struct FnPublishCallback {
    on_message: Box<dyn FnMut(MessageRef) + Send + 'static>,
}

impl FnPublishCallback {
    fn new(on_message: impl FnMut(MessageRef) + Send + 'static) -> Self {
        Self {
            on_message: Box::new(on_message),
        }
    }

    fn into_ffi(self) -> PublishCallback {
        unsafe extern "C" fn on_publish(
            _connection: *mut aws_mqtt_client_connection,
            topic: *const aws_byte_cursor,
            payload: *const aws_byte_cursor,
            dup: bool,
            qos: aws_mqtt_qos,
            retain: bool,
            userdata: *mut c_void,
        ) {
            let topic = ByteCursor::from_ptr(topic);
            let payload = ByteCursor::from_ptr(payload);
            let userdata = unsafe { &mut *userdata.cast::<FnPublishCallback>() };

            let msg = MessageRef {
                // SAFETY: mqtt topics are always UTF-8 encoded
                // TODO: verify that the mqtt library actually checks this!
                topic: core::str::from_utf8_unchecked(topic.as_bytes()),
                payload: payload.as_bytes(),
                dup,
                qos: Qos(qos),
                retain,
            };
            (userdata.on_message)(msg);
        }

        unsafe extern "C" fn cleanup_userdata(userdata: *mut c_void) {
            let userdata = Box::<FnPublishCallback>::from_raw(userdata.cast());
            // deallocate trait object
            drop(userdata);
        }

        let userdata = Box::new(self);
        PublishCallback {
            on_publish: Some(on_publish),
            userdata: Box::into_raw(userdata).cast(),
            cleanup_userdata: Some(cleanup_userdata),
        }
    }
}
