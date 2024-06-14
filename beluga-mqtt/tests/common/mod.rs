use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use beluga_mqtt::{MqttClient, MqttClientBuilder};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, DnValue, Ia5String, IsCa, KeyPair,
    KeyUsagePurpose, SanType,
};
use rumqttd::{
    Broker, Config, ConnectionSettings, RouterConfig, ServerSettings, ShutdownHandler, TlsConfig,
};
use time::{Duration, OffsetDateTime};

pub fn ca() -> anyhow::Result<(Certificate, KeyPair)> {
    let mut params = CertificateParams::new(Vec::default())?;

    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.distinguished_name.push(
        DnType::CountryName,
        DnValue::PrintableString("UA".try_into()?),
    );
    params
        .distinguished_name
        .push(DnType::OrganizationName, "Beluga organization");
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];

    let day = Duration::new(86400, 0);
    params.not_before = OffsetDateTime::now_utc()
        .checked_sub(day)
        .context("couldn't shift a day backward")?;
    params.not_after = OffsetDateTime::now_utc()
        .checked_add(day)
        .context("couldn't shift a day forward")?;

    let key = KeyPair::generate()?;
    Ok((params.self_signed(&key)?, key))
}

pub fn signed_cert(ca: &Certificate, ca_key: &KeyPair) -> anyhow::Result<(Certificate, KeyPair)> {
    let name = "localhost";
    let mut params = CertificateParams::new(vec![name.into()])?;

    let day = Duration::new(86400, 0);
    params.not_before = OffsetDateTime::now_utc()
        .checked_sub(day)
        .context("couldn't couldn't shift a day backward")?;
    params.not_after = OffsetDateTime::now_utc()
        .checked_add(day)
        .context("couldn't shift a day forward")?;
    params.distinguished_name.push(DnType::CommonName, name);
    params.use_authority_key_identifier_extension = true;
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params.subject_alt_names = vec![
        SanType::DnsName(Ia5String::from_str(name)?),
        SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))),
    ];

    let key = KeyPair::generate()?;
    let cert = params.signed_by(&key, ca, ca_key)?;

    Ok((cert, key))
}

pub fn mqtt_server(ca: String, certpath: String, keypath: String, port: u16) -> ShutdownHandler {
    let settings = ServerSettings {
        name: "mqtt-server".to_owned(),
        listen: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port)),
        tls: Some(TlsConfig::Rustls {
            capath: Some(ca),
            certpath,
            keypath,
        }),
        next_connection_delay_ms: 500,
        connections: ConnectionSettings {
            connection_timeout_ms: 1000,
            max_payload_size: 2048,
            max_inflight_count: 15,
            auth: None,
            external_auth: None,
            dynamic_filters: false,
        },
    };

    let mut v4_map = HashMap::new();
    v4_map.insert("-".to_owned(), settings);

    let config = Config {
        router: RouterConfig {
            max_connections: 15,
            max_outgoing_packet_count: 10,
            max_segment_size: 2048,
            max_segment_count: 50,
            ..Default::default()
        },
        v4: Some(v4_map),
        ..Default::default()
    };

    let mut broker = Broker::new(config);
    let handler = broker.shutdown_handler();
    std::thread::spawn(move || {
        broker.start().unwrap();
    });

    handler
}

pub fn client(
    ca: &Certificate,
    cert: &Certificate,
    key: &KeyPair,
    port: u16,
    name: &str,
) -> anyhow::Result<MqttClient> {
    Ok(MqttClientBuilder::new()
        .ca(ca.pem().as_bytes())
        .certificate(cert.pem().as_bytes())
        .private_key(key.serialize_pem().as_bytes())
        .thing_name(name)
        .endpoint("127.0.0.1")
        .port(port)
        .build()?)
}

pub fn port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub async fn server_certs(
    tmp_dir: &Path,
    ca: &Certificate,
    ca_key: &KeyPair,
) -> anyhow::Result<(PathBuf, PathBuf, PathBuf)> {
    let (server_cert, server_key) = signed_cert(ca, ca_key)?;

    let ca_path = tmp_dir.join("ca.pem");
    let cert_path = tmp_dir.join("cert.pem");
    let key_path = tmp_dir.join("key.pem");

    tokio::fs::write(&ca_path, ca.pem()).await?;
    tokio::fs::write(&cert_path, server_cert.pem()).await?;
    tokio::fs::write(&key_path, server_key.serialize_pem()).await?;

    Ok((ca_path, cert_path, key_path))
}
