use aws_crt::io::{ClientBootstrap, EventLoopGroup, HostResolver, SocketOptions};
use aws_crt::mqtt::{Client, ConnectionOptionsBuilder, Qos};
use aws_crt::{Allocator, ApiHandle};

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    futures::executor::block_on(_main());
}

async fn _main() {
    let _handle = ApiHandle::get();

    let el_group = EventLoopGroup::new_default(Allocator::default(), 1).unwrap();
    let host_resolver = HostResolver::builder(&el_group).build().unwrap();
    let bootstrap = ClientBootstrap::builder(&el_group, &host_resolver)
        .build()
        .unwrap();
    let client = Client::new(Allocator::default(), &bootstrap).unwrap();
    let connection = client.create_connection().unwrap();

    let socket_options = SocketOptions::builder().build();

    let resp = connection
        .connect(
            ConnectionOptionsBuilder::new()
                .client_id("12b80a000")
                .host_name("test.mosquitto.org")
                .port(1883)
                .socket_options(&socket_options),
        )
        .await
        .unwrap();
    log::info!("connected: {resp:?}");

    loop {
        let res = connection
            .publish("12b80a000/test", Qos::AT_MOST_ONCE, false, b"hello world")
            .await;
        log::info!("publish: {res:?}");
    }
}
