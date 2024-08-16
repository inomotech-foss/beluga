use std::collections::HashMap;

use beluga_mqtt::{MqttClient, QoS};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use data::{
    CreateCertificateFromCsrReq, CreateCertificateFromCsrResp, CreateKeysAndCertificateResp,
    RegisterThingReq,
};
use payload::{from_bytes, to_bytes};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::Result;

mod data;

pub use data::{DeviceCertificateInfo, ProvisionError, RegisterThingResponse};

/// Creates a new device certificate and keys using the MQTT client.
///
/// If the request is accepted, the function returns a `DeviceCertificateInfo`
/// struct containing the new certificate information. If the request is
/// rejected, the function returns a `ProvisionError`.
///
/// # Arguments
/// * `client` - The MQTT client to use for the request.
///
/// # Returns
/// A `Result` containing the `DeviceCertificateInfo` if the request is
/// accepted, or a `ProvisionError` if the request is rejected.
pub async fn create_keys_and_certificate(client: MqttClient) -> Result<DeviceCertificateInfo> {
    let mut accepted = client
        .subscribe_owned(topic::CREATE_KEYS_AND_CERT_ACCEPTED, QoS::AtLeastOnce)
        .await?;
    let mut rejected = client
        .subscribe_owned(topic::CREATE_KEYS_AND_CERT_REJECTED, QoS::AtLeastOnce)
        .await?;

    client
        .publish(
            topic::CREATE_KEYS_AND_CERT_REQ,
            QoS::AtLeastOnce,
            false,
            Bytes::new(),
        )
        .await?;

    tokio::select! {
        res = accepted.recv() => {
            let packet = res?;
            Ok(from_bytes::<CreateKeysAndCertificateResp>(packet.payload)?.into())
        },
        res = rejected.recv() => {
            let packet = res?;
            Err(from_bytes::<ProvisionError>(packet.payload)?.into())
        }
    }
}

/// Creates a new device certificate and keys using the provided **CSR
/// (Certificate Signing Request)**.
///
/// If the request is accepted, the function returns a `DeviceCertificateInfo`
/// struct containing the new certificate information. If the request is
/// rejected, the function returns a `ProvisionError`.
///
/// # Arguments
/// * `client` - The MQTT client to use for the request.
/// * `csr` - The **CSR (Certificate Signing Request)** to use for creating the
///   new certificate.
///
/// # Returns
/// A `Result` containing the `DeviceCertificateInfo` if the request is
/// accepted, or a `ProvisionError` if the request is rejected.
pub async fn create_certificate_from_csr(
    client: MqttClient,
    csr: String,
) -> Result<DeviceCertificateInfo> {
    let mut accepted = client
        .subscribe_owned(topic::CREATE_FROM_CSR_ACCEPTED, QoS::AtLeastOnce)
        .await?;
    let mut rejected = client
        .subscribe_owned(topic::CREATE_FROM_CSR_REJECTED, QoS::AtLeastOnce)
        .await?;

    client
        .publish(
            topic::CREATE_FROM_CSR_REQUEST,
            QoS::AtLeastOnce,
            false,
            payload::to_bytes(&CreateCertificateFromCsrReq { csr })?,
        )
        .await?;

    tokio::select! {
        res = accepted.recv() => {
            let packet = res?;
            Ok(from_bytes::<CreateCertificateFromCsrResp>(packet.payload)?.into())
        },
        res = rejected.recv() => {
            let packet = res?;
            Err(from_bytes::<ProvisionError>(packet.payload)?.into())
        }
    }
}

/// Registers a new device with the AWS IoT Core service using the provided
/// device certificate information and provisioning template.
///
/// # Arguments
/// * `client` - The MQTT client to use for the registration request.
/// * `info` - The `DeviceCertificateInfo` containing the certificate and keys
///   to use for the registration.
/// * `template_name` - The name of the provisioning template to use for the
///   registration.
/// * `parameters` - Optional parameters to pass to the provisioning template.
///
/// # Returns
/// A `Result` containing the `RegisterThingResponse` if the registration is
/// successful, or a `ProvisionError` if the registration is rejected.
pub async fn register_thing<Iter: IntoIterator<Item = (String, String)>>(
    client: MqttClient,
    info: &DeviceCertificateInfo,
    template_name: impl AsRef<str>,
    parameters: Option<Iter>,
) -> Result<RegisterThingResponse> {
    let mut accepted = client
        .subscribe_owned(
            topic::register_thing_accepted(template_name.as_ref()),
            QoS::AtLeastOnce,
        )
        .await?;

    let mut rejected = client
        .subscribe_owned(
            topic::register_thing_rejected(template_name.as_ref()),
            QoS::AtLeastOnce,
        )
        .await?;

    let parameters = if let Some(params) = parameters {
        params.into_iter().collect::<HashMap<_, _>>()
    } else {
        HashMap::new()
    };

    client
        .publish(
            topic::register_thing_request(template_name.as_ref()),
            QoS::AtLeastOnce,
            false,
            to_bytes(&RegisterThingReq {
                ownership_token: info.ownership_token.clone(),
                parameters,
            })?,
        )
        .await?;

    tokio::select! {
        res = accepted.recv() => {
            let packet = res?;
            Ok(from_bytes::<RegisterThingResponse>(packet.payload)?)
        },
        res = rejected.recv() => {
            let packet = res?;
            Err(from_bytes::<ProvisionError>(packet.payload)?.into())
        }
    }
}

#[cfg(feature = "cbor")]
mod topic {
    pub(super) const CREATE_FROM_CSR_REQUEST: &str = "$aws/certificates/create-from-csr/cbor";
    pub(super) const CREATE_FROM_CSR_ACCEPTED: &str =
        "$aws/certificates/create-from-csr/cbor/accepted";
    pub(super) const CREATE_FROM_CSR_REJECTED: &str =
        "$aws/certificates/create-from-csr/cbor/rejected";
    pub(super) const CREATE_KEYS_AND_CERT_REQ: &str = "$aws/certificates/create/cbor";
    pub(super) const CREATE_KEYS_AND_CERT_ACCEPTED: &str = "$aws/certificates/create/cbor/accepted";
    pub(super) const CREATE_KEYS_AND_CERT_REJECTED: &str = "$aws/certificates/create/cbor/rejected";

    #[must_use]
    #[inline(always)]
    pub(super) fn register_thing_request(template: &str) -> String {
        format!("$aws/provisioning-templates/{template}/provision/cbor")
    }

    #[must_use]
    #[inline(always)]
    pub(super) fn register_thing_accepted(template: &str) -> String {
        format!("$aws/provisioning-templates/{template}/provision/cbor/accepted")
    }

    #[must_use]
    #[inline(always)]
    pub(super) fn register_thing_rejected(template: &str) -> String {
        format!("$aws/provisioning-templates/{template}/provision/cbor/rejected")
    }
}

#[cfg(not(feature = "cbor"))]
mod topic {
    pub(super) const CREATE_FROM_CSR_REQUEST: &str = "$aws/certificates/create-from-csr/json";
    pub(super) const CREATE_FROM_CSR_ACCEPTED: &str =
        "$aws/certificates/create-from-csr/json/accepted";
    pub(super) const CREATE_FROM_CSR_REJECTED: &str =
        "$aws/certificates/create-from-csr/json/rejected";
    pub(super) const CREATE_KEYS_AND_CERT_REQ: &str = "$aws/certificates/create/json";
    pub(super) const CREATE_KEYS_AND_CERT_ACCEPTED: &str = "$aws/certificates/create/json/accepted";
    pub(super) const CREATE_KEYS_AND_CERT_REJECTED: &str = "$aws/certificates/create/json/rejected";

    #[must_use]
    #[inline(always)]
    pub(super) fn register_thing_request(template: &str) -> String {
        format!("$aws/provisioning-templates/{template}/provision/json")
    }

    #[must_use]
    #[inline(always)]
    pub(super) fn register_thing_accepted(template: &str) -> String {
        format!("$aws/provisioning-templates/{template}/provision/json/accepted")
    }

    #[must_use]
    #[inline(always)]
    pub(super) fn register_thing_rejected(template: &str) -> String {
        format!("$aws/provisioning-templates/{template}/provision/json/rejected")
    }
}

#[cfg(feature = "cbor")]
mod payload {
    use super::*;
    use crate::error::CborError;

    #[allow(clippy::result_large_err)]
    pub(super) fn to_bytes<T>(value: &T) -> Result<Bytes>
    where
        T: ?Sized + Serialize,
    {
        let mut writer = BytesMut::new().writer();
        ciborium::into_writer(value, &mut writer).map_err(CborError::from)?;
        Ok(writer.into_inner().freeze())
    }

    #[allow(clippy::result_large_err)]
    pub(super) fn from_bytes<T: DeserializeOwned>(buf: Bytes) -> Result<T> {
        let value = ciborium::from_reader(buf.reader()).map_err(CborError::from)?;
        Ok(value)
    }
}

#[cfg(not(feature = "cbor"))]
mod payload {
    use super::*;

    #[allow(clippy::result_large_err)]
    pub(super) fn to_bytes<T>(value: &T) -> Result<Bytes>
    where
        T: ?Sized + Serialize,
    {
        let mut writer = BytesMut::new().writer();
        serde_json::to_writer(&mut writer, value)?;
        Ok(writer.into_inner().freeze())
    }

    #[allow(clippy::result_large_err)]
    pub(super) fn from_bytes<T: DeserializeOwned>(buf: Bytes) -> Result<T> {
        Ok(serde_json::from_reader(buf.reader())?)
    }
}
