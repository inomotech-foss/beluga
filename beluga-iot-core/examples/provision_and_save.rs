use std::collections::HashMap;

use anyhow::Context;
use beluga_aws_sdk::provision::{
    create_certificate_from_csr, register_thing, RegisterThingResponse,
};
use beluga_mqtt::MqttClientBuilder;
use itertools::Itertools;
use petname::Generator;
use rcgen::{CertificateParams, CertificateSigningRequest, IsCa, KeyPair};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{error, Level};

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(alias = "endpoint")]
    endpoint: String,
    #[serde(alias = "thing")]
    #[serde(alias = "thing_name")]
    thing_name: String,
    #[serde(alias = "ca")]
    #[serde(alias = "ca_path")]
    ca: String,
    #[serde(alias = "template")]
    #[serde(alias = "provision_template")]
    template: String,
    #[serde(alias = "device")]
    #[serde(alias = "device_certificate")]
    device: CertificateInfo,
    #[serde(alias = "provision")]
    #[serde(alias = "provision_certificate")]
    provision: CertificateInfo,
}

#[derive(Debug, Deserialize)]
struct CertificateInfo {
    #[serde(alias = "cert")]
    #[serde(alias = "cert_path")]
    certificate: String,
    #[serde(alias = "key")]
    #[serde(alias = "key_path")]
    key: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let config_path = std::env::args()
        .skip(1)
        .next_tuple::<(String, String)>()
        .and_then(|(arg, path)| (arg == "-c" || arg == "--config").then_some(path));

    let Some(path) = config_path else {
        error!("configuration path not provided with \"-c\" or \"--config\" argument");
        return Ok(());
    };

    let config = match toml::from_str::<Config>(&tokio::fs::read_to_string(path).await?) {
        Ok(config) => config,
        Err(err) => {
            error!("couldn't parse the config cause of \"{err}\"");
            return Ok(());
        }
    };

    let client = MqttClientBuilder::new()
        .ca(&tokio::fs::read(&config.ca).await?)
        .certificate(&tokio::fs::read(&config.provision.certificate).await?)
        .private_key(&tokio::fs::read(&config.provision.key).await?)
        .endpoint(&config.endpoint)
        .thing_name("it doesn't matter")
        .build()?;

    let (csr, key_pair) = new_csr()?;

    let mut params = HashMap::new();
    params.insert(
        "ThingUUID".to_owned(),
        format!(
            "{}-{}",
            uuid::Uuid::new_v4(),
            petname::Petnames::default()
                .generate_one(3, "-")
                .context("couldn't configure custom name")?
        ),
    );

    let info = create_certificate_from_csr(client.clone(), csr.pem()?).await?;
    let RegisterThingResponse { thing_name, .. } =
        register_thing(client.clone(), &info, &config.template, Some(params)).await?;

    tokio::fs::write(config.thing_name, thing_name).await?;
    tokio::fs::write(config.device.certificate, info.certificate).await?;
    tokio::fs::write(config.device.key, key_pair.serialize_pem()).await?;

    Ok(())
}

fn new_csr() -> anyhow::Result<(CertificateSigningRequest, KeyPair)> {
    let mut params = CertificateParams::new(Vec::default())?;
    params.is_ca = IsCa::NoCa;

    let past = OffsetDateTime::now_utc()
        .checked_sub(time::Duration::days(2))
        .context("couldn't subtract two days")?;
    let future = OffsetDateTime::now_utc()
        .checked_add(time::Duration::days(2))
        .context("couldn't add two days")?;

    params.not_before = past;
    params.not_after = future;

    let key_pair = KeyPair::generate()?;
    let csr = params.serialize_request(&key_pair)?;

    Ok((csr, key_pair))
}
