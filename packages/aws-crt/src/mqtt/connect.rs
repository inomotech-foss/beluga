use core::ffi::{c_int, c_void};
use core::marker::PhantomData;
use core::time::Duration;

use aws_c_mqtt_sys::{
    aws_mqtt_client_connection, aws_mqtt_client_connection_connect, aws_mqtt_connect_return_code,
    aws_mqtt_connection_options,
};

use super::Connection;
use crate::io::SocketOptions;
use crate::mqtt::TaskFuture;
use crate::{ByteCursor, Error};

pub struct ConnectionOptionsBuilder<'a> {
    options: aws_mqtt_connection_options,
    // struct holds hidden references
    _marker: PhantomData<&'a ()>,
}

impl ConnectionOptionsBuilder<'static> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            options: aws_mqtt_connection_options {
                host_name: ByteCursor::empty().into_inner(),
                port: 0,
                socket_options: core::ptr::null_mut(), // required
                tls_options: core::ptr::null_mut(),
                client_id: ByteCursor::empty().into_inner(),
                keep_alive_time_secs: 0,
                ping_timeout_ms: 0,
                protocol_operation_timeout_ms: 0,
                on_connection_complete: None,
                user_data: core::ptr::null_mut(),
                clean_session: false,
            },
            _marker: PhantomData,
        }
    }
}

impl<'a> ConnectionOptionsBuilder<'a> {
    fn as_ptr(&self) -> *const aws_mqtt_connection_options {
        &self.options
    }

    /// The server name to connect to.
    pub fn host_name(&mut self, value: &'a str) -> &mut Self {
        self.options.host_name = ByteCursor::from_str(value).into_inner();
        self
    }

    /// The port on the server to connect to.
    pub fn port(&mut self, value: u32) -> &mut Self {
        self.options.port = value;
        self
    }

    /// The clientid to place in the CONNECT packet.
    pub fn client_id(&mut self, value: &'a str) -> &mut Self {
        self.options.client_id = ByteCursor::from_str(value).into_inner();
        self
    }

    /// The keep alive value to place in the CONNECT PACKET.
    ///
    /// Granularity of seconds. Will saturate to `u16::MAX` seconds (more than
    /// 18 days). Defaults to 20 minutes if unset or set to 0.
    ///    
    /// A PING will automatically be sent at this interval as well. This
    /// duration must be longer than `ping_timeout`.
    pub fn keep_alive_time(&mut self, value: Duration) -> &mut Self {
        self.options.keep_alive_time_secs = value.as_secs().try_into().unwrap_or(u16::MAX);
        self
    }

    /// Network connection is re-established if a ping response is not received
    /// within this amount of time.
    ///
    /// Granularity of milliseconds. Will saturate to `u32::MAX` milliseconds
    /// (almost 50 days). Defaults to 3 seconds if unset or set to 0.
    ///
    /// Alternatively, tcp keep-alive may be a way to accomplish this in
    /// a more efficient (low-power) scenario, but keep-alive options may
    /// not work the same way on every platform and OS version. This
    /// duration must be shorter than `keep_alive_time`.
    pub fn ping_timeout(&mut self, value: Duration) -> &mut Self {
        self.options.ping_timeout_ms = value.as_millis().try_into().unwrap_or(u32::MAX);
        self
    }

    /// Timeout when waiting for the response to some operation requires
    /// response by protocol.
    ///
    /// Granularity of milliseconds. Will saturate to `u32::MAX` milliseconds
    /// (almost 50 days). Defaults to 0 which disables the timeout entirely.
    ///
    /// If enabled, the operation will fail with a timeout error if no response
    /// is received within this amount of time after the packet is written
    /// to the socket. The timer is reset if the connection is interrupted.
    /// It applied to PUBLISH (QoS>0) and UNSUBSCRIBE now.
    ///
    /// Note: While the MQTT 3 specification states that a broker MUST respond,
    /// some brokers are known to ignore publish packets in exceptional
    /// circumstances (e.g. AWS IoT Core will not respond if the publish
    /// quota is exceeded).
    pub fn protocol_operation_timeout(&mut self, value: Duration) -> &mut Self {
        self.options.protocol_operation_timeout_ms =
            value.as_millis().try_into().unwrap_or(u32::MAX);
        self
    }

    /// True to discard all server session data and start fresh.
    pub fn clean_session(&mut self, value: bool) -> &mut Self {
        self.options.clean_session = value;
        self
    }

    pub fn socket_options(&mut self, options: &'a SocketOptions) -> &mut Self {
        self.options.socket_options = options.as_ptr();
        self
    }
}

impl Connection {
    pub fn connect(
        &self,
        options: &mut ConnectionOptionsBuilder,
    ) -> TaskFuture<ConnectionCompleteData> {
        debug_assert!(!options.options.socket_options.is_null());
        debug_assert!(!options.options.client_id.len > 0); // TODO: does this apply to v5 as well?

        unsafe extern "C" fn on_connection_complete(
            _connection: *mut aws_mqtt_client_connection,
            error_code: c_int,
            return_code: aws_mqtt_connect_return_code,
            session_present: bool,
            userdata: *mut c_void,
        ) {
            let res = Error::check_rc(error_code).map(|()| ConnectionCompleteData {
                return_code: ConnectReturnCode(return_code),
                session_present,
            });
            TaskFuture::resolve(userdata, res);
        }

        if options.options.socket_options.is_null() {
            todo!()
        }

        let (resolver, fut) = crate::future::create();

        options.options.on_connection_complete = Some(on_connection_complete);
        options.options.user_data = resolver.into_raw();

        let res = Error::check_rc(unsafe {
            aws_mqtt_client_connection_connect(self.as_ptr(), options.as_ptr())
        });
        TaskFuture::create(res, fut)
    }
}

#[derive(Debug)]
pub struct ConnectionCompleteData {
    pub return_code: ConnectReturnCode,
    pub session_present: bool,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ConnectReturnCode(aws_mqtt_connect_return_code);
