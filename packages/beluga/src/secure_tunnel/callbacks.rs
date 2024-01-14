use alloc::boxed::Box;
use core::ffi::c_void;

use aws_c_iot_sys::{
    aws_secure_tunnel_connection_view, aws_secure_tunnel_message_type,
    aws_secure_tunnel_message_view, aws_secure_tunnel_options,
    aws_secure_tunneling_on_termination_complete_fn,
};

use crate::Error;

pub trait Callbacks {
    fn message_received(&self, message: &MessageView);
    fn connection_complete(&self, connection: &ConnectionView, error: Error);
    fn connection_shutdown(&self, error: Error);
    fn send_message_complete(&self, message_type: MessageType, error: Error);
    fn stream_start(&self, message: &MessageView, error: Error);
    fn stream_reset(&self, message: &MessageView, error: Error);
    fn connection_start(&self, message: &MessageView, error: Error);
    fn connection_reset(&self, message: &MessageView, error: Error);
    fn session_reset(&self);
    fn stopped(&self);
}

#[repr(transparent)]
pub struct MessageView(aws_secure_tunnel_message_view);

#[repr(transparent)]
pub struct ConnectionView(aws_secure_tunnel_connection_view);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MessageType(aws_secure_tunnel_message_type);

struct UserData {
    callbacks: Box<dyn Callbacks>,
}

impl UserData {
    fn apply(self: Box<Self>, options: &mut aws_secure_tunnel_options) {
        let (on_termination_complete, user_data) = self.into_ffi();
        options.on_termination_complete = on_termination_complete;
        options.user_data = user_data;
        options.secure_tunnel_on_termination_user_data = user_data;
    }

    fn into_ffi(self: Box<Self>) -> (aws_secure_tunneling_on_termination_complete_fn, *mut c_void) {
        extern "C" fn on_termination_complete(user_data: *mut c_void) {
            let user_data = unsafe { Box::from_raw(user_data.cast::<UserData>()) };
            drop(user_data);
        }

        let user_data = Box::into_raw(self);
        (Some(on_termination_complete), user_data.cast())
    }
}
