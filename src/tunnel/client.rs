use std::ffi::{c_char, c_void, CString};
use std::io::{self, ErrorKind};
use std::net::SocketAddrV4;
use std::ops::Deref;
use std::sync::Arc;

use futures::future::BoxFuture;
use futures::{future, Future, FutureExt};
use parking_lot::{const_fair_mutex, const_mutex, FairMutex, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, Mutex as TokioMutex, Notify};
use tokio::task::JoinHandle;
use tracing::debug;

use super::callbacks::{
    create_connection_failure_callback, create_connection_reset_callback,
    create_connection_shutdown_callback, create_connection_success_callback,
    create_message_received_callback, create_send_message_complete_callback,
    create_session_reset_callback, create_stream_started_callback, create_stream_stopped_callback,
    create_subscribe_complete_callback, create_subscribe_tunnel_callback, ClientInterface,
    Credentials, TunnelInterface,
};
use crate::common::{Buffer, SharedPtr, UniquePtr};
use crate::mqtt::{InternalMqttClient, MqttClient};
use crate::{ApiHandle, Error, Qos, Result};

extern "C" {
    fn internal_tunnel_client(
        mqtt_client: *const InternalMqttClient,
        interface: *const c_void,
        qos: Qos,
        thing_name: *const c_char,
    ) -> *const InternalTunnelClient;

    fn internal_tunnel(
        interface: *const c_void,
        endpoint: *const c_char,
        access_token: *const c_char,
    ) -> *const InternalTunnel;

    fn send_message(tunnel: *const InternalTunnel, connection_id: u32, payload: Buffer) -> i32;

    fn drop_internal_tunnel_client(tunnel_client: *const InternalTunnelClient);
    fn drop_internal_tunnel(tunnel: *const InternalTunnel);
    fn start(tunnel: *const InternalTunnel) -> i32;
    fn stop(tunnel: *const InternalTunnel) -> i32;
}

#[repr(C)]
pub(super) struct InternalTunnelClient {
    client: UniquePtr,
}

#[repr(C)]
pub(super) struct InternalTunnel {
    tunnel: SharedPtr,
}

#[derive(Debug)]
pub(super) struct InternalTunnelClientPointer {
    internal_client: *const InternalTunnelClient,
}

impl Deref for InternalTunnelClientPointer {
    type Target = *const InternalTunnelClient;
    fn deref(&self) -> &Self::Target {
        &self.internal_client
    }
}

unsafe impl Send for InternalTunnelClientPointer {}
unsafe impl Sync for InternalTunnelClientPointer {}

#[derive(Debug)]
pub(super) struct InternalTunnelPointer {
    tunnel: *const InternalTunnel,
}

impl Deref for InternalTunnelPointer {
    type Target = *const InternalTunnel;
    fn deref(&self) -> &Self::Target {
        &self.tunnel
    }
}

unsafe impl Send for InternalTunnelPointer {}
unsafe impl Sync for InternalTunnelPointer {}

#[derive(Debug, Clone)]
pub(super) struct Message {
    pub(super) _connection_id: u32,
    pub(super) payload: Vec<u8>,
}

pub struct TunnelClient {
    internal_client: Arc<Mutex<InternalTunnelClientPointer>>,
    _mqtt_client: Arc<MqttClient>,
    _interface: Arc<ClientInterface>,
    sessions: Arc<FairMutex<Vec<JoinHandle<Result<()>>>>>,
    drop_client: Arc<Notify>,
    task: JoinHandle<()>,
}

impl Drop for TunnelClient {
    fn drop(&mut self) {
        self.drop_client.notify_waiters();

        for session in self.sessions.lock().drain(..) {
            session.abort();
        }

        // let's stop background task
        self.task.abort();

        unsafe {
            let mut guard = self.internal_client.lock();
            // drop tunnel client
            drop_internal_tunnel_client(guard.internal_client);
            guard.internal_client = std::ptr::null();
        }
    }
}

impl TunnelClient {
    pub async fn create(mqtt_client: Arc<MqttClient>, qos: Qos, thing_name: &str) -> Result<Self> {
        ApiHandle::handle();

        let (create_client_tx, mut create_client_rx) = mpsc::channel::<i32>(1);
        let (tunnel_created_tx, tunnel_created_rx) = mpsc::channel::<Credentials>(10);

        let client_interface = Arc::new(ClientInterface {
            subscribe_complete: Box::new(create_subscribe_complete_callback(create_client_tx)),
            subscribe_tunnel: Box::new(create_subscribe_tunnel_callback(tunnel_created_tx)),
        });

        let thing_name = CString::new(thing_name).map_err(Error::StringConversion)?;

        let internal_client = {
            let internal_client = mqtt_client.internal_client();
            let client = internal_client.lock();
            Arc::new(const_mutex(InternalTunnelClientPointer {
                internal_client: unsafe {
                    internal_tunnel_client(
                        **client,
                        (client_interface.as_ref() as *const ClientInterface).cast(),
                        qos,
                        thing_name.as_ptr(),
                    )
                },
            }))
        };

        if internal_client.lock().is_null() {
            return Err(Error::TunnelClientCreate);
        }

        let sessions = Arc::new(const_fair_mutex(Vec::new()));
        let drop_client = Arc::new(Notify::new());

        let tunnel_client = TunnelClient {
            internal_client,
            _mqtt_client: mqtt_client,
            _interface: client_interface,
            task: tokio::spawn(subscribe(
                tunnel_created_rx,
                sessions.clone(),
                drop_client.clone(),
            )),
            sessions,
            drop_client,
        };

        match create_client_rx.recv().await {
            Some(error_code) => {
                if error_code != 0 {
                    Err(Error::TunnelClientCreate)
                } else {
                    Ok(tunnel_client)
                }
            }
            None => Err(Error::TunnelClientCreate),
        }
    }
}

async fn subscribe(
    mut tunnel_created_rx: mpsc::Receiver<Credentials>,
    sessions: Arc<FairMutex<Vec<JoinHandle<Result<()>>>>>,
    drop_client_notify: Arc<Notify>,
) {
    loop {
        let sessions = Arc::clone(&sessions);
        let drop_client_notify = Arc::clone(&drop_client_notify);
        if let Some(Credentials {
            access_token,
            region,
            ..
        }) = tunnel_created_rx.recv().await
        {
            let mut sessions = sessions.lock();
            // Removes all finished session handles from the sessions vector.
            // This ensures only active sessions remain in the sessions list.
            sessions.retain(|handle| !handle.is_finished());
            sessions.push(tokio::spawn(tunnel(
                format!("data.tunneling.iot.{region}.amazonaws.com"),
                access_token,
                drop_client_notify.clone(),
            )));
        }
    }
}

async fn tunnel(endpoint: String, access_token: String, drop_client: Arc<Notify>) -> Result<()> {
    let (shutdown_tx, shutdown_rx) = shutdown_channels();
    let (message_receive_tx, message_receive_rx) = mpsc::channel::<Message>(20);
    let (stream_start_tx, mut stream_start_rx) = mpsc::channel::<u32>(1);

    let tunnel_interface = TunnelInterface {
        connection_success: Box::new(create_connection_success_callback()),
        connection_failure: Box::new(create_connection_failure_callback(
            shutdown_tx.connection_failure_tx,
        )),
        connection_shutdown: Box::new(create_connection_shutdown_callback(
            shutdown_tx.connection_shutdown,
        )),
        connection_reset: Box::new(create_connection_reset_callback(
            shutdown_tx.connection_reset_tx,
        )),
        session_reset: Box::new(create_session_reset_callback(shutdown_tx.session_reset)),
        send_message_complete: Box::new(create_send_message_complete_callback()),
        message_received: Box::new(create_message_received_callback(message_receive_tx)),
        stream_started: Box::new(create_stream_started_callback(stream_start_tx)),
        stream_stopped: Box::new(create_stream_stopped_callback(shutdown_tx.stream_stop)),
    };

    let endpoint = CString::new(endpoint).map_err(Error::StringConversion)?;
    let access_token = CString::new(access_token).map_err(Error::StringConversion)?;

    let internal_tunnel = Arc::new(TokioMutex::new(InternalTunnelPointer {
        tunnel: unsafe {
            internal_tunnel(
                (&tunnel_interface as *const TunnelInterface).cast(),
                endpoint.as_ptr(),
                access_token.as_ptr(),
            )
        },
    }));

    if unsafe { start(internal_tunnel.lock().await.tunnel) } != 0 {
        drop_tunnel(internal_tunnel.clone(), shutdown_rx.cancel()).await;
        return Err(Error::TunnelCreate);
    }

    let Some(connection_id) = stream_start_rx.recv().await else {
        drop_tunnel(internal_tunnel.clone(), shutdown_rx.cancel()).await;
        return Err(Error::TunnelCreate);
    };

    debug!(%connection_id, "stream started");

    let cancel = future::select_all([
        erase_return(shutdown_rx.cancel()),
        erase_return(drop_client.notified()),
    ]);

    future::select_all([
        erase_return(ssh(
            internal_tunnel.clone(),
            22,
            connection_id,
            message_receive_rx,
        )),
        erase_return(cancel),
    ])
    .await;

    drop_tunnel(internal_tunnel.clone(), shutdown_rx.cancel()).await;
    debug!("exited from tunnel");

    Ok(())
}

async fn ssh(
    internal_tunnel: Arc<TokioMutex<InternalTunnelPointer>>,
    port: u16,
    connection_id: u32,
    mut packets: mpsc::Receiver<Message>,
) -> Result<()> {
    let stream = TcpStream::connect::<SocketAddrV4>(SocketAddrV4::new([127, 0, 0, 1].into(), port))
        .await
        .map_err(Error::IoError)?;

    let (mut reader, mut writer) = stream.into_split();

    let tunnel = internal_tunnel.clone();
    let reader_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        let mut buff = [0; 1024];

        loop {
            let tunnel = tunnel.clone();
            let size = reader.read(&mut buff).await?;

            if size == 0 {
                return Ok(());
            }

            let send_res = {
                let tunnel_ptr = tunnel.lock_owned().await;
                unsafe {
                    send_message(
                        tunnel_ptr.tunnel,
                        connection_id,
                        Buffer::from(&buff[0..size]),
                    )
                }
            };

            if send_res != 0 {
                return Err(Error::IoError(io::Error::new(
                    ErrorKind::Other,
                    "couldn't send a message",
                )));
            }
        }
    });

    let writer_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        while let Some(packet) = packets.recv().await {
            writer.write_all(&packet.payload).await?;
        }
        Ok(())
    });

    let _ = future::try_join(reader_task, writer_task)
        .await
        .map_err(|_err| {
            Error::IoError(io::Error::new(ErrorKind::Other, "ssh connection failed"))
        })?;

    Ok(())
}

async fn drop_tunnel(
    internal_tunnel: Arc<TokioMutex<InternalTunnelPointer>>,
    wait_till_stop: impl Future<Output = ()>,
) {
    unsafe { stop(internal_tunnel.lock().await.tunnel) };
    wait_till_stop.await;
    unsafe { drop_internal_tunnel(internal_tunnel.lock().await.tunnel) };
}

fn shutdown_channels() -> (CancelSender, CancelReceiver) {
    let (connection_failure_tx, connection_failure_rx) = broadcast::channel::<i32>(1);
    let (connection_reset_tx, connection_reset_rx) =
        broadcast::channel::<(i32, u32, Option<String>)>(1);
    let connection_shutdown = Arc::new(Notify::new());
    let session_reset = Arc::new(Notify::new());
    let stream_stop = Arc::new(Notify::new());

    let receiver = CancelReceiver {
        connection_failure_rx,
        connection_reset_rx,
        connection_shutdown: connection_shutdown.clone(),
        session_reset: session_reset.clone(),
        stream_stop: stream_stop.clone(),
    };

    let sender = CancelSender {
        connection_failure_tx,
        connection_reset_tx,
        connection_shutdown,
        session_reset,
        stream_stop,
    };

    (sender, receiver)
}

struct CancelReceiver {
    connection_failure_rx: broadcast::Receiver<i32>,
    connection_reset_rx: broadcast::Receiver<(i32, u32, Option<String>)>,
    connection_shutdown: Arc<Notify>,
    session_reset: Arc<Notify>,
    stream_stop: Arc<Notify>,
}

impl CancelReceiver {
    fn cancel(&self) -> impl Future<Output = ()> {
        let mut connection_failure = self.connection_failure_rx.resubscribe();
        let mut connection_reset = self.connection_reset_rx.resubscribe();
        let connection_shutdown = self.connection_shutdown.clone();
        let session_reset = self.session_reset.clone();
        let stream_stop = self.stream_stop.clone();

        async move {
            erase_return(future::select_all([
                erase_return(connection_failure.recv()),
                erase_return(connection_reset.recv()),
                erase_return(connection_shutdown.notified()),
                erase_return(session_reset.notified()),
                erase_return(stream_stop.notified()),
            ]))
            .await;
        }
    }
}

struct CancelSender {
    connection_failure_tx: broadcast::Sender<i32>,
    connection_reset_tx: broadcast::Sender<(i32, u32, Option<String>)>,
    connection_shutdown: Arc<Notify>,
    session_reset: Arc<Notify>,
    stream_stop: Arc<Notify>,
}

fn erase_return<'a, T>(future: impl Future<Output = T> + Send + 'a) -> BoxFuture<'a, ()> {
    future.map(|_| ()).boxed()
}
