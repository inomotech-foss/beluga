use std::future::Future;
use std::pin::Pin;
use std::result::Result as StdResult;
use std::sync::Arc;
use std::task::{Context, Poll};

use pin_project::pin_project;
use tokio::sync::oneshot::error::RecvError;

use super::{ClientStatus, Message, MqttClient};
use crate::{AwsMqttError, Error, Result};

/// The `CreateMqttFuture` struct represents a future for creating an MQTT
/// client.
#[pin_project]
pub struct CreateMqttFuture<F>
where
    F: Future<Output = StdResult<ClientStatus, RecvError>>,
{
    client: Option<MqttClient>,
    #[pin]
    receiver: F,
}

impl<F> CreateMqttFuture<F>
where
    F: Future<Output = StdResult<ClientStatus, RecvError>>,
{
    pub(super) fn new(client: MqttClient, receiver: F) -> Self {
        Self {
            client: Some(client),
            receiver,
        }
    }
}

impl<F> Future for CreateMqttFuture<F>
where
    F: Future<Output = StdResult<ClientStatus, RecvError>>,
{
    type Output = Result<Arc<MqttClient>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let poll = this.receiver.poll(cx);

        match poll {
            Poll::Ready(status) => {
                if let Ok(ClientStatus::Connected) = status {
                    this.client
                        .take()
                        .map(|client| Poll::Ready(Ok(Arc::new(client))))
                        .unwrap_or(Poll::Ready(Err(Error::MqttClientCreate)))
                } else {
                    Poll::Ready(Err(Error::MqttClientCreate))
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// The [`SubscribeMessageFuture`] wraps a future that produces a [`Message`].
#[pin_project]
pub struct SubscribeMessageFuture<F>
where
    F: Future<Output = StdResult<Message, RecvError>>,
{
    #[pin]
    receiver: F,
}

impl<F> SubscribeMessageFuture<F>
where
    F: Future<Output = StdResult<Message, RecvError>>,
{
    pub(super) fn new(receiver: F) -> Self {
        Self { receiver }
    }
}

impl<F> Future for SubscribeMessageFuture<F>
where
    F: Future<Output = StdResult<Message, RecvError>>,
{
    type Output = Result<Message>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().receiver.poll(cx) {
            Poll::Ready(Ok(msg)) => Poll::Ready(Ok(msg)),
            Poll::Ready(Err(_)) => Poll::Ready(Err(Error::AwsReceiveMessage)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// The `OperationResponseFuture` represents the future that will resolve to the
/// result of an operation.
#[pin_project]
pub struct OperationResponseFuture<F>
where
    F: Future<Output = StdResult<i32, RecvError>>,
{
    #[pin]
    receiver: F,
}

impl<F> OperationResponseFuture<F>
where
    F: Future<Output = StdResult<i32, RecvError>>,
{
    pub(super) fn new(receiver: F) -> Self {
        Self { receiver }
    }
}

impl<F> Future for OperationResponseFuture<F>
where
    F: Future<Output = StdResult<i32, RecvError>>,
{
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().receiver.poll(cx) {
            Poll::Ready(Ok(0)) => Poll::Ready(Ok(())),
            Poll::Ready(Ok(error_code)) => Poll::Ready(Err(AwsMqttError::try_from(error_code)
                .map(Error::from)
                .unwrap_or(Error::AwsUnknownMqttError(error_code)))),
            Poll::Ready(Err(_)) => Poll::Ready(Err(Error::AwsReceiveResponse)),
            Poll::Pending => Poll::Pending,
        }
    }
}
