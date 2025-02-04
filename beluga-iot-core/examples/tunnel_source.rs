use std::env;
use std::time::Duration;

use anyhow::Context;
use aws_runtime::env_config::file::EnvConfigFiles;
use aws_sdk_iotsecuretunneling::types::DestinationConfig;
use beluga_ssh_service::SshService;
use beluga_tunnel::{ClientMode, Notify, Tunnel};
use openssh::{KnownHosts, SessionBuilder};

const SERVICES: &str = "SSH";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv_override();
    let key = env::var("SSH_PRIV_KEY").context("Missing SSH_PRIV_KEY")?;
    let region = env::var("AWS_REGION").context("Missing AWS_REGION")?;
    let thing_name = env::var("AWS_THING_NAME").context("Missing AWS_THING_NAME")?;
    let user = env::var("REMOTE_USER").unwrap_or("root".to_string());

    let config = aws_config::from_env()
        .profile_files(EnvConfigFiles::default())
        .load()
        .await;
    let client = aws_sdk_iotsecuretunneling::Client::new(&config);

    let config = DestinationConfig::builder()
        .thing_name(thing_name)
        .services(SERVICES)
        .build()?;
    let manager = client
        .open_tunnel()
        .destination_config(config)
        .send()
        .await?;
    println!(
        "Created tunnel {}",
        manager.tunnel_id().context("Empty tunnel ID")?,
    );

    let notify = Notify::new(
        manager
            .source_access_token()
            .context("Empty source access token")?,
        ClientMode::Source,
        region,
        vec![SERVICES.to_string()],
    );
    let tunnel = Tunnel::new(&notify).await?;
    let service = SshService::source();
    let handle = tokio::spawn(async move {
        if let Err(e) = tunnel.start(service).await {
            eprintln!("Tunnel closed unexpectedly : {e}");
        }
    });

    let session = SessionBuilder::default()
        .port(service.port())
        .user(user)
        .keyfile(key)
        .connect_timeout(Duration::from_secs(10))
        .known_hosts_check(KnownHosts::Accept)
        .connect("localhost")
        .await
        .context("Failed to start SSH session")?;

    let version = session.command("uname").arg("-a").output().await?;
    println!("{}", String::from_utf8(version.stdout)?.trim());

    if let Err(e) = session.close().await {
        eprintln!("Failed to stop SSH session : {e}");
    }

    handle.abort();

    client
        .close_tunnel()
        .tunnel_id(manager.tunnel_id().unwrap())
        .delete(true)
        .send()
        .await?;

    println!("Tunnel closed");

    Ok(())
}
