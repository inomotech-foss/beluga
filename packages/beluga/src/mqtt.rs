use core::ffi::{c_int, c_void};

use aws_c_common_sys::aws_byte_cursor;
use aws_c_mqtt_sys::{
    aws_mqtt_client, aws_mqtt_client_acquire, aws_mqtt_client_connection,
    aws_mqtt_client_connection_acquire, aws_mqtt_client_connection_disconnect,
    aws_mqtt_client_connection_new, aws_mqtt_client_connection_publish,
    aws_mqtt_client_connection_release, aws_mqtt_client_connection_set_login,
    aws_mqtt_client_connection_set_reconnect_timeout, aws_mqtt_client_connection_set_will,
    aws_mqtt_client_connection_subscribe, aws_mqtt_client_new, aws_mqtt_client_release,
    aws_mqtt_qos, AWS_MQTT_QOS_AT_LEAST_ONCE, AWS_MQTT_QOS_AT_MOST_ONCE, AWS_MQTT_QOS_EXACTLY_ONCE,
};

pub use self::connect::*;
pub use self::futures::{PacketFuture, TaskFuture};
use self::subscribe::PublishCallback;
pub use self::subscribe::SubscribeAck;
use crate::io::ClientBootstrap;
use crate::{AllocatorRef, ByteCursor, Error, Result};

mod connect;
mod futures;
mod subscribe;

ref_counted_wrapper!(struct ClientInner(aws_mqtt_client) {
    acquire: aws_mqtt_client_acquire,
    release: aws_mqtt_client_release,
});

#[derive(Clone)]
#[repr(transparent)]
pub struct Client(ClientInner);

impl Client {
    pub fn new(allocator: AllocatorRef, bootstrap: &ClientBootstrap) -> Result<Self> {
        unsafe {
            ClientInner::new_or_error(aws_mqtt_client_new(allocator.as_ptr(), bootstrap.as_ptr()))
        }
        .map(Self)
    }

    #[must_use]
    pub const fn as_ptr(&self) -> *mut aws_mqtt_client {
        self.0.as_ptr()
    }

    pub fn create_connection(&self) -> Result<Connection> {
        Connection::new(self)
    }
}

ref_counted_wrapper!(struct ConnectionInner(aws_mqtt_client_connection) {
    acquire: aws_mqtt_client_connection_acquire,
    release: aws_mqtt_client_connection_release,
});

#[derive(Clone)]
#[repr(transparent)]
pub struct Connection(ConnectionInner);

impl Connection {
    fn new(client: &Client) -> Result<Self> {
        unsafe { ConnectionInner::new_or_error(aws_mqtt_client_connection_new(client.as_ptr())) }
            .map(Self)
    }

    #[must_use]
    pub const fn as_ptr(&self) -> *mut aws_mqtt_client_connection {
        self.0.as_ptr()
    }

    pub fn set_will(&self, topic: &str, qos: Qos, retain: bool, payload: &[u8]) -> Result<()> {
        let topic = ByteCursor::from_str(topic);
        let payload = ByteCursor::from_slice(payload);
        Error::check_rc(unsafe {
            aws_mqtt_client_connection_set_will(
                self.as_ptr(),
                topic.as_ptr(),
                qos.0,
                retain,
                payload.as_ptr(),
            )
        })
    }

    pub fn set_login(&self, username: &str, password: &str) -> Result<()> {
        let username = ByteCursor::from_str(username);
        let password = ByteCursor::from_str(password);
        Error::check_rc(unsafe {
            aws_mqtt_client_connection_set_login(
                self.as_ptr(),
                username.as_ptr(),
                password.as_ptr(),
            )
        })
    }

    pub fn set_reconnect_timeout(&self, min_seconds: u64, max_seconds: u64) -> Result<()> {
        Error::check_rc(unsafe {
            aws_mqtt_client_connection_set_reconnect_timeout(
                self.as_ptr(),
                min_seconds,
                max_seconds,
            )
        })
    }

    pub fn disconnect(&self) -> TaskFuture<()> {
        unsafe extern "C" fn on_disconnect(
            _connection: *mut aws_mqtt_client_connection,
            userdata: *mut c_void,
        ) {
            TaskFuture::<()>::resolve(userdata, Ok(()));
        }

        let (resolver, fut) = crate::future::create();
        let res = Error::check_rc(unsafe {
            aws_mqtt_client_connection_disconnect(
                self.as_ptr(),
                Some(on_disconnect),
                resolver.into_raw(),
            )
        });
        TaskFuture::create(res, fut)
    }

    pub fn publish(&self, topic: &str, qos: Qos, retain: bool, payload: &[u8]) -> PacketFuture<()> {
        unsafe extern "C" fn on_complete(
            _connection: *mut aws_mqtt_client_connection,
            _packet_id: u16,
            error_code: c_int,
            userdata: *mut c_void,
        ) {
            PacketFuture::<()>::resolve_with_error_code(userdata, error_code);
        }

        let topic = ByteCursor::from_str(topic);
        let payload = ByteCursor::from_slice(payload);
        let (resolver, fut) = crate::future::create();
        let packet_id = unsafe {
            aws_mqtt_client_connection_publish(
                self.as_ptr(),
                topic.as_ptr(),
                qos.0,
                retain,
                payload.as_ptr(),
                Some(on_complete),
                resolver.into_raw(),
            )
        };
        PacketFuture::create(packet_id, fut)
    }

    fn subscribe_impl(
        &self,
        topic_filter: &str,
        qos: Qos,
        publish_callback: PublishCallback,
    ) -> PacketFuture<SubscribeAck> {
        unsafe extern "C" fn on_suback(
            _connection: *mut aws_mqtt_client_connection,
            _packet_id: u16,
            _topic: *const aws_byte_cursor,
            qos: aws_mqtt_qos,
            error_code: c_int,
            userdata: *mut c_void,
        ) {
            let res = Error::check_rc(error_code).map(|()| SubscribeAck {
                granted_qos: Qos(qos),
            });
            PacketFuture::<SubscribeAck>::resolve(userdata, res);
        }

        let topic_filter = ByteCursor::from_str(topic_filter);
        let (resolver, fut) = crate::future::create();
        let packet_id = unsafe {
            aws_mqtt_client_connection_subscribe(
                self.as_ptr(),
                topic_filter.as_ptr(),
                qos.0,
                publish_callback.on_publish,
                publish_callback.userdata,
                publish_callback.cleanup_userdata,
                Some(on_suback),
                resolver.into_raw(),
            )
        };
        if packet_id == 0 {
            // SAFETY: if the call fails the userdata isn't cleaned up so we manually call
            // the cleanup function here.
            unsafe { publish_callback.cleanup() };
        }
        PacketFuture::create(packet_id, fut)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Qos(pub(crate) aws_mqtt_qos);

impl Qos {
    pub const AT_LEAST_ONCE: Self = Self(AWS_MQTT_QOS_AT_LEAST_ONCE);
    pub const AT_MOST_ONCE: Self = Self(AWS_MQTT_QOS_AT_MOST_ONCE);
    pub const EXACTLY_ONCE: Self = Self(AWS_MQTT_QOS_EXACTLY_ONCE);
}
