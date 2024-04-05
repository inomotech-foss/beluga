use anyhow::{bail, Context};
use bytes::{Buf, BufMut};
use core::net::SocketAddrV4;
use enumn::N;
use futures_util::{future, pin_mut, SinkExt, StreamExt, TryStreamExt};
use http::Request;
use rumqttc::{AsyncClient, MqttOptions, Packet, QoS, TlsConfiguration, Transport};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, Mutex},
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::{handshake::client::generate_key, Message};

const AWS_IOT_ST_FIELD_NUMBER_SHIFT: u32 = 3;
const AWS_IOT_ST_MAXIMUM_VARINT: u32 = 268435455;
const AWS_IOT_ST_MAXIMUM_1_BYTE_VARINT_VALUE: u32 = 128;
const AWS_IOT_ST_MAXIMUM_2_BYTE_VARINT_VALUE: u32 = 16384;
const AWS_IOT_ST_MAXIMUM_3_BYTE_VARINT_VALUE: u32 = 2097152;
const AWS_IOT_ST_MAX_PAYLOAD_SIZE: u32 = 63 * 1024;

/*
{
    "clientAccessToken": "destination-client-access-token",
    "clientMode": "destination",
    "region": "aws-region",
    "services": ["destination-service"]
}
*/

#[derive(Debug, Deserialize, Serialize)]
struct Notify {
    #[serde(rename = "clientAccessToken")]
    client_access_token: String,
    #[serde(rename = "clientMode")]
    client_mode: String,
    region: String,
    services: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut mqttoptions = MqttOptions::new(
        "thing-name",
        "endpoint",
        8883,
    );
    mqttoptions.set_keep_alive(std::time::Duration::from_secs(5));

    let ca = include_bytes!("../AmazonRootCA1.pem");
    let client_cert = include_bytes!("../certificate.pem.crt");
    let client_key = include_bytes!("../private.pem.key");

    let transport = Transport::Tls(TlsConfiguration::Simple {
        ca: ca.to_vec(),
        alpn: None,
        client_auth: Some((client_cert.to_vec(), client_key.to_vec())),
    });

    mqttoptions.set_transport(transport);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    client
        .subscribe(
            "$aws/things/thing-name/tunnels/notify",
            QoS::AtLeastOnce,
        )
        .await?;

    loop {
        match eventloop.poll().await {
            Ok(v) => {
                // println!("Event = {v:?}");

                if let rumqttc::Event::Incoming(Packet::Publish(p)) = v {
                    let Notify {
                        client_access_token,
                        client_mode,
                        region,
                        services,
                    } = serde_json::from_slice::<Notify>(&p.payload).unwrap();

                    let req = Request::builder()
                        .method("GET")
                        .header("access-token", client_access_token)
                        // .header(":path", "/tunnel?local-proxy-mode=1") // dst mode
                        // .header("client-token", "some")
                        .header(http::header::HOST, format!("data.tunneling.iot.{region}.amazonaws.com"))
                        .header("Connection", "Upgrade")
                        .header("Upgrade", "websocket")
                        .header("Sec-WebSocket-Version", "13")
                        .header("Sec-WebSocket-Key", generate_key())
                        .header("Sec-WebSocket-Protocol", "aws.iot.securetunneling-3.0")
                        .uri(format!("wss://data.tunneling.iot.{region}.amazonaws.com/tunnel?local-proxy-mode=destination"))
                        // .uri("data.tunneling.iot.{region}.amazonaws.com/tunnel?local-proxy-mode=1")
                        // .header(key, value)
                        // .uri(format!("data.tunneling.iot.{region}.amazonaws.com"))
                        .body(())?;

                    tokio::spawn(async move {
                        let (stream, resp) =
                            tokio_tungstenite::connect_async_tls_with_config(req, None, true, None)
                                .await
                                .unwrap();

                        println!("Response {resp:?}");
                        println!("client mode {client_mode:?}");

                        let (mut write, mut read) = stream.split();

                        let (tx_out, mut rx_out) = mpsc::channel::<Vec<u8>>(100);
                        let (tx_in, rx_in) = mpsc::channel::<Vec<u8>>(100);

                        let tx_out = Arc::new(Mutex::new(tx_out));
                        let rx_in = Arc::new(Mutex::new(rx_in));

                        loop {
                            tokio::select! {
                                data = rx_out.recv() => {
                                    let msg = data.unwrap();
                                    write.send(Message::Binary(msg)).await.unwrap();
                                }
                                msg = read.next() => {
                                    if let Some(Ok(msg)) = msg {
                                        let mut bytes = bytes::Bytes::from_iter(msg.into_data());
                                        if let Ok(messages) = process_received_data(&mut bytes) {
                                            for msg in messages {
                                                println!("Message: {:?}", msg);
                                                if let Some(MessageType::Data) = msg.msg_type {
                                                    let data = msg.payload.unwrap();
                                                    tx_in.send(data.to_vec()).await.unwrap();
                                                }

                                                if let Some(MessageType::StreamStart) = msg.msg_type {
                                                    ssh(tx_out.clone(), rx_in.clone()).await;
                                                    println!("Stream start");
                                                }
                                            }
                                        }



                                    }
                                }

                            }
                        }
                    });
                }
            }
            Err(e) => {
                println!("Error = {e:?}");
                break;
            }
        }
    }

    Ok(())
}

async fn ssh(tx: Arc<Mutex<mpsc::Sender<Vec<u8>>>>, rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>) {
    tokio::spawn(async move {
        let stream =
            TcpStream::connect::<SocketAddrV4>(SocketAddrV4::new([127, 0, 0, 1].into(), 22))
                .await
                .unwrap();

        let (mut reader, mut writer) = stream.into_split();
        let mut buff = [0; 1024];

        loop {
            // let data = rx.recv().await.unwrap();
            // println!("Write packet {:?}", &data);
            // writer.write_all(&data).await.unwrap();

            let size = reader.read(&mut buff).await.unwrap();

            if size == 0 {
                continue;
            }

            println!("Read SSH packet {:?}", &buff[..size]);

            let msg = MessageView {
                msg_type: MessageType::Data.into(),
                ignorable: false.into(),
                stream_id: 5.into(),
                connection_id: None,
                service_id: "SSH".to_owned().into(),
                service_id2: None,
                service_id3: None,
                payload: bytes::Bytes::copy_from_slice(&buff[..size]).into(),
            };

            let data = serialize_data(msg).unwrap();
            tx.lock().await.send(data.to_vec()).await.unwrap();

            let data = rx.lock().await.recv().await.unwrap();
            println!("Write packet {:?}", &data);
            writer.write_all(&data).await.unwrap();
        }
    });
}

#[derive(Debug, PartialEq, PartialOrd, N)]
#[repr(u8)]
enum FieldNumber {
    Type = 1,
    StreamId = 2,
    Ignorable = 3,
    Payload = 4,
    ServiceId = 5,
    AvailableServiceIds = 6,
    ConnectionId = 7,
}

#[derive(Debug, PartialEq, PartialOrd, N)]
#[repr(u8)]
enum WireType {
    Varint = 0,
    Size64 = 1,
    LengthDelimited = 2,
    StartGroup = 3,
    EndGroup = 4,
    Size32 = 5,
}

/**
 * Type of IoT Secure Tunnel message.
 * Enum values match IoT Secure Tunneling Local Proxy V3 Websocket Protocol Guide values.
 *
 * https://github.com/aws-samples/aws-iot-securetunneling-localproxy/blob/main/V3WebSocketProtocolGuide.md
*/
#[derive(Debug, PartialEq, PartialOrd, N, Clone, Copy)]
#[repr(u32)]
enum MessageType {
    Unknown = 0,
    /**
     * Data messages carry a payload with a sequence of bytes to write to the the active data stream
     */
    Data = 1,
    /**
     * StreamStart is the first message sent to start and establish a new and active data stream. This should only be
     * sent from a Source to a Destination.
     */
    StreamStart = 2,
    /**
     * StreamReset messages convey that the data stream has ended, either in error, or closed intentionally for the
     * tunnel peer. It is also sent to the source tunnel peer if an attempt to establish a new data stream fails on the
     * destination side.
     */
    StreamReset = 3,
    /**
     * SessionReset messages can only originate from Secure Tunneling service if an internal data transmission error is
     * detected. This will result in all active streams being closed.
     */
    SessionReset = 4,
    /**
     * ServiceIDs messages can only originate from the Secure Tunneling service and carry a list of unique service IDs
     * used when opening a tunnel with services.
     */
    ServiceIds = 5,
    /**
     * ConnectionStart is the message sent to start and establish a new and active connection when the stream has been
     * established and there's one active connection in the stream.
     */
    ConnectionStart = 6,
    /**
     * ConnectionReset messages convey that the connection has ended, either in error, or closed intentionally for the
     * tunnel peer. These should not be manually sent from either Destination or Source clients.
     */
    ConnectionReset = 7,
}

#[derive(Debug, Default)]
struct MessageView {
    msg_type: Option<MessageType>,
    ignorable: Option<bool>,
    stream_id: Option<i32>,
    connection_id: Option<u32>,
    service_id: Option<String>,
    service_id2: Option<String>,
    service_id3: Option<String>,
    payload: Option<bytes::Bytes>,
}

fn decode_u32(buf: &mut bytes::Bytes) -> anyhow::Result<u32> {
    let mut value = 0_u32;
    let mut position = 0_u32;

    while !buf.is_empty() {
        let current_byte = buf.get_u8();
        value |= ((current_byte & 0x7F) << position) as u32;

        if current_byte & 0x80 == 0 {
            return Ok(value);
        }

        position += 7;

        if position >= 32 {
            bail!("VarInt is too big")
        }
    }

    Ok(value)
}

fn encode_u32(buf: &mut bytes::BytesMut, mut value: u32) {
    // & 2's comp   lement
    // ~0x7F == b-10000000
    while value & !0x7F > 0 {
        buf.put_u8(((value & 0xFF) | 0x80) as u8);
        value = value >> 7;
    }
    buf.put_u8(value as u8);
}

fn encode_u32_neg(buf: &mut bytes::BytesMut, mut value: u32) {
    let mut byte_count = 0;
    while value & !0x7F > 0 {
        value >>= 7;
        byte_count += 1;
    }

    // Last Byte Math
    let mut count = 0;
    while (!(value & 0x80)) > 0 {
        value = value << 1;
        count += 1;
    }

    for i in 0..count {
        value = value >> 1;
        value = value | 0x80;
    }

    buf.put_u8(value as u8);
    for i in 0..(10 - byte_count - 2) {
        buf.put_u8(0xFF);
    }
    buf.put_u8(0x1);
}

fn encode_u32_pos(buf: &mut bytes::BytesMut, value: i32) {
    if value >= 0 {
        encode_u32(buf, value as u32);
    } else if value < 0 {
        encode_u32_neg(buf, value as u32);
    }
}

fn encode_varint(buf: &mut bytes::BytesMut, field_number: u8, wire_type: u8, value: i32) {
    let field_and_wire_type = (field_number << AWS_IOT_ST_FIELD_NUMBER_SHIFT) + wire_type;
    buf.put_u8(field_and_wire_type);
}

fn encode_byte_range(buf: &mut bytes::BytesMut, field_number: u8, wire_type: u8, payload: &[u8]) {
    let field_and_wire_type = (field_number << AWS_IOT_ST_FIELD_NUMBER_SHIFT) + wire_type;
    buf.put_u8(field_and_wire_type);
    encode_u32(buf, payload.len() as u32);
    buf.put_slice(payload);
}

fn encode_stream_id(buf: &mut bytes::BytesMut, data: i32) {
    encode_varint(
        buf,
        FieldNumber::StreamId as u8,
        WireType::Varint as u8,
        data,
    );
}

fn encode_connection_id(buf: &mut bytes::BytesMut, data: u32) {
    encode_varint(
        buf,
        FieldNumber::ConnectionId as u8,
        WireType::Varint as u8,
        data as i32,
    );
}

fn encode_ignorable(buf: &mut bytes::BytesMut, data: i32) {
    encode_varint(
        buf,
        FieldNumber::Ignorable as u8,
        WireType::Varint as u8,
        data,
    );
}

fn encode_type(buf: &mut bytes::BytesMut, data: i32) {
    encode_varint(
        buf,
        FieldNumber::Ignorable as u8,
        WireType::Varint as u8,
        data,
    );
}

fn encode_payload(buf: &mut bytes::BytesMut, payload: &[u8]) {
    encode_byte_range(
        buf,
        FieldNumber::Payload as u8,
        WireType::LengthDelimited as u8,
        payload,
    );
}

fn encode_service_id(buf: &mut bytes::BytesMut, service_id: String) {
    encode_byte_range(
        buf,
        FieldNumber::ServiceId as u8,
        WireType::LengthDelimited as u8,
        service_id.as_bytes(),
    );
}

fn encode_service_ids(buf: &mut bytes::BytesMut, service_id: String) {
    encode_byte_range(
        buf,
        FieldNumber::AvailableServiceIds as u8,
        WireType::LengthDelimited as u8,
        service_id.as_bytes(),
    );
}

fn get_varint_size(value: u32) -> anyhow::Result<usize> {
    if value > AWS_IOT_ST_MAXIMUM_VARINT {
        bail!("Reached maximum varint size")
    }

    if value < AWS_IOT_ST_MAXIMUM_1_BYTE_VARINT_VALUE {
        Ok(1)
    } else if value < AWS_IOT_ST_MAXIMUM_2_BYTE_VARINT_VALUE {
        Ok(2)
    } else if value < AWS_IOT_ST_MAXIMUM_3_BYTE_VARINT_VALUE {
        Ok(3)
    } else {
        Ok(4)
    }
}

fn process_received_data(data: &mut bytes::Bytes) -> anyhow::Result<Vec<MessageView>> {
    let mut messages = vec![];

    let mut data_length = 0;
    while data.len() >= core::mem::size_of::<u16>() && data.len() >= data_length {
        data_length = data.get_u16().into();

        let mut msg = MessageView::default();

        while !data.is_empty() {
            let next_byte = data.get_u8();
            let wire_type = WireType::n(next_byte & 0x07).context(format!(
                "couldn't parse wire type from: \"{}\"",
                next_byte & 0x07
            ))?;
            let field_number = FieldNumber::n(next_byte >> 3).context(format!(
                "couldn't parse field number from: \"{}\"",
                next_byte >> 3
            ))?;

            match wire_type {
                WireType::Varint => {
                    let result = decode_u32(data)?;
                    match field_number {
                        FieldNumber::Type => {
                            msg.msg_type = MessageType::n(result).context("wef")?.into();
                        }
                        FieldNumber::StreamId => {
                            msg.stream_id = Some(result as i32);
                        }
                        FieldNumber::Ignorable => {
                            msg.ignorable = Some(result > 0);
                        }
                        FieldNumber::ConnectionId => {
                            msg.connection_id = Some(result);
                        }
                        _ => {}
                    }
                }
                WireType::LengthDelimited => {
                    let length = decode_u32(data)? as usize;

                    match field_number {
                        FieldNumber::Type => todo!(),
                        FieldNumber::StreamId => todo!(),
                        FieldNumber::Ignorable => todo!(),
                        FieldNumber::Payload => {
                            msg.payload = bytes::Bytes::copy_from_slice(&data[..length]).into();
                        }
                        FieldNumber::ServiceId => {
                            msg.service_id =
                                core::str::from_utf8(&data[..length])?.to_owned().into();
                        }
                        FieldNumber::AvailableServiceIds => {
                            if msg.service_id.is_none() {
                                msg.service_id =
                                    core::str::from_utf8(&data[..length])?.to_owned().into();
                            } else if msg.service_id2.is_none() {
                                msg.service_id2 =
                                    core::str::from_utf8(&data[..length])?.to_owned().into();
                            } else if msg.service_id3.is_none() {
                                msg.service_id3 =
                                    core::str::from_utf8(&data[..length])?.to_owned().into();
                            }
                        }
                        _ => {}
                    }
                    data.advance(length as usize);
                }
                typ => {
                    bail!("Unexpected wire type in message encountered. {typ:?}")
                }
            }
        }

        messages.push(msg);
    }

    Ok(messages)
}

fn compute_message_length(msg: &MessageView) -> anyhow::Result<usize> {
    /*
     * 1 byte type key
     * 1 byte type varint
     */
    let mut local_length = 2;

    if let Some(stream_id) = msg.stream_id {
        /*
         * 1 byte stream_id key
         * 1-4 byte stream_id varint
         */
        let stream_id_length = get_varint_size(stream_id as u32)?;
        local_length += 1 + stream_id_length;
    }

    if let Some(connection_id) = msg.connection_id {
        /*
         * 1 byte connection_id key
         * 1-4 byte connection_id varint
         */
        let connection_id_length = get_varint_size(connection_id)?;
        local_length += 1 + connection_id_length;
    }

    if msg.ignorable.is_some() {
        /*
         * 1 byte ignorable key
         * 1 byte ignorable varint
         */
        local_length += 2;
    }

    if let Some(payload) = msg.payload.as_ref() {
        /*
         * 1 byte key
         * 1-4 byte payload length varint
         * n bytes payload.len
         */
        let payload_length = get_varint_size(payload.len() as u32)?;
        local_length += 1 + payload.len() + payload_length;
    }

    if let Some(service_id) = msg.service_id.as_ref() {
        /*
         * 1 byte key
         * 1-4 byte payload length varint
         * n bytes service_id.len
         */
        let service_id_length = get_varint_size(service_id.len() as u32)?;
        local_length += 1 + service_id.len() + service_id_length;
    }

    if let Some(service_id2) = msg.service_id2.as_ref() {
        /*
         * 1 byte key
         * 1-4 byte payload length varint
         * n bytes service_id.len
         */
        let service_id_length = get_varint_size(service_id2.len() as u32)?;
        local_length += 1 + service_id2.len() + service_id_length;
    }

    if let Some(service_id3) = msg.service_id3.as_ref() {
        /*
         * 1 byte key
         * 1-4 byte payload length varint
         * n bytes service_id.len
         */
        let service_id_length = get_varint_size(service_id3.len() as u32)?;
        local_length += 1 + service_id3.len() + service_id_length;
    }

    Ok(local_length)
}

fn serialize_data(msg: MessageView) -> anyhow::Result<bytes::Bytes> {
    let msg_length = compute_message_length(&msg)?;
    let mut buf = bytes::BytesMut::with_capacity(msg_length);

    let Some(typ) = msg.msg_type else {
        bail!("Message missing type during encoding");
    };

    encode_type(&mut buf, typ as i32);

    if let Some(stream_id) = msg.stream_id {
        encode_stream_id(&mut buf, stream_id);
    }

    if let Some(connection_id) = msg.connection_id {
        encode_stream_id(&mut buf, connection_id as i32);
    }

    if let Some(ignorable) = msg.ignorable {
        encode_ignorable(&mut buf, ignorable as i32);
    }

    if let Some(payload) = msg.payload {
        encode_payload(&mut buf, &payload);
    }

    if let MessageType::ServiceIds = typ {
        if let Some(service_id) = msg.service_id {
            encode_service_ids(&mut buf, service_id);
        }

        if let Some(service_id2) = msg.service_id2 {
            encode_service_ids(&mut buf, service_id2);
        }

        if let Some(service_id3) = msg.service_id3 {
            encode_service_ids(&mut buf, service_id3);
        }
    } else if let Some(service_id) = msg.service_id {
        encode_service_id(&mut buf, service_id);
    }

    Ok(buf.freeze())
}
