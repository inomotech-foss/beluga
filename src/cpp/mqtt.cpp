#include "mqtt.h"
#include "logs.h"

extern "C" void on_completed(const void *, int, ReturnCode, bool);
extern "C" void on_closed(const void *);
extern "C" void on_interrupted(const void *, int);
extern "C" void on_resumed(const void *, ReturnCode, bool);
extern "C" void on_message(const void *, const char *, Buffer, bool, QOS, bool);
extern "C" void on_sub_ack(const void *, uint16_t, const char *, QOS, int);
extern "C" void on_publish(const void *, uint16_t, int);
extern "C" void on_unsubscribe(const void *, uint16_t, int);

InternalMqttClient::InternalMqttClient(std::shared_ptr<MqttConnection> connection, const void *interface)
    : connection(connection), interface(interface) {}

std::shared_ptr<MqttConnection> InternalMqttClient::get_connection() const
{
    return this->connection;
}

const void *InternalMqttClient::get_interface() const
{
    return this->interface;
}

extern "C" InternalMqttClient *internal_mqtt_client(ClientConfig client_config, const void *interface)
{
    debug("start building internal mqtt client");

    auto config_builder = MqttBuilder();
    if (!client_config.certificate.is_empty() && !client_config.private_key.is_empty())
    {
        config_builder = MqttBuilder(
            Aws::Crt::ByteCursorFromByteBuf(client_config.certificate.into()),
            Aws::Crt::ByteCursorFromByteBuf(client_config.private_key.into())
        );
    }
    else if (strlen(client_config.username) != 0 && strlen(client_config.password) != 0)
    {
        config_builder.WithPassword(AwsString(client_config.password));
        config_builder.WithUsername(AwsString(client_config.username));
    }
    else
    {
        error("config is missing password auth or pub/priv key auth");
        return nullptr;
    }

    config_builder.WithEndpoint(AwsString(client_config.endpoint));
    if (client_config.port != 0)
    {
        config_builder.WithPortOverride(client_config.port);
    }

    const auto config = config_builder.Build();
    if (!config)
    {
        error("couldn't build a config for internal mqtt client");
        return nullptr;
    }

    auto client = Aws::Iot::MqttClient();
    auto connection = client.NewConnection(config);
    if (!*connection)
    {
        error("couldn't create an internal mqtt client");
        return nullptr;
    }

    connection->OnConnectionCompleted = [=](MqttConnection &, int error_code, ReturnCode return_code, bool session_present)
    {
        debug("internal mqtt client: on completed");
        on_completed(interface, error_code, return_code, session_present);
    };

    connection->OnConnectionClosed = [=](MqttConnection &, Aws::Crt::Mqtt::OnConnectionClosedData *)
    {
        debug("internal mqtt client: on closed");
        on_closed(interface);
    };

    connection->OnConnectionInterrupted = [=](MqttConnection &, int error)
    {
        debug("internal mqtt client: on interrupted");
        on_interrupted(interface, error);
    };

    connection->OnConnectionResumed = [=](MqttConnection &, ReturnCode return_code, bool session_present)
    {
        debug("internal mqtt client: on resumed");
        on_resumed(interface, return_code, session_present);
    };

    if (!connection->Connect(client_config.client_id, client_config.clean_session, client_config.keep_alive_s, client_config.ping_timeout_ms))
    {
        error(format("error during connect: client_id:[%s], endpoint:[%s], last error:[%s]",
                     client_config.client_id, client_config.endpoint, connection->LastError())
                  .c_str());
        return nullptr;
    }

    return new InternalMqttClient(connection, interface);
}

/**
 * Subscribes to topic.
 *
 * @param topic topic filter to subscribe to
 * @param qos maximum qos client is willing to receive matching messages on
 *
 * @return packet id of the subscribe request, or 0 if the attempt failed synchronously
 */
extern "C" uint16_t subscribe(InternalMqttClient *client, const char *topic, QOS qos)
{
    auto connection = client->get_connection();
    return connection->Subscribe(
        topic, qos,
        [=](MqttConnection &, const AwsString &topic, const ByteBuf &payload, bool dup, QOS qos, bool retain)
        {
            on_message(client->get_interface(), topic.c_str(), Buffer(payload), dup, qos, retain);
        },
        [=](MqttConnection &, uint16_t packet_id, const AwsString &topic, QOS qos, int error_code)
        {
            on_sub_ack(client->get_interface(), packet_id, topic.c_str(), qos, error_code);
        }
    );
}

/**
 * Subscribes to multiple topics.
 *
 * @param topics topics filter to subscribe to
 * @param topics_len length of the topics array
 * @param qos maximum qos client is willing to receive matching messages on
 *
 * @return packet id of the subscribe request, or 0 if the attempt failed synchronously
 */
extern "C" uint16_t subscribe_multiple(InternalMqttClient *client, const char *const *topics, size_t topics_len, QOS qos)
{
    auto topics_vector = Aws::Crt::Vector<std::pair<const char *, Aws::Crt::Mqtt::OnMessageReceivedHandler>>();
    for (size_t i = 0; i < topics_len; ++i)
    {
        topics_vector.push_back(
            std::make_pair(
                topics[i],
                [=](MqttConnection &, const AwsString &topic, const ByteBuf &payload, bool dup, QOS qos, bool retain)
                {
                    on_message(client->get_interface(), topic.c_str(), Buffer(payload), dup, qos, retain);
                }
            )
        );
    }

    auto connection = client->get_connection();
    return connection->Subscribe(
        topics_vector, qos,
        [=](MqttConnection &, uint16_t packet_id, const Aws::Crt::Vector<AwsString> &topics, QOS qos, int error_code)
        {
            for (const auto &topic : topics)
            {
                on_sub_ack(client->get_interface(), packet_id, topic.c_str(), qos, error_code);
            }
        }
    );
}

/**
 * Unsubscribes from topic. lambda will be invoked upon receipt of
 * an unsuback message.
 *
 * @param topic topic filter to unsubscribe the session from
 *
 * @return packet id of the unsubscribe request, or 0 if the attempt failed synchronously
 */
extern "C" uint16_t unsubscribe(InternalMqttClient *client, const char *topic)
{
    auto connection = client->get_connection();
    return connection->Unsubscribe(
        topic,
        [=](MqttConnection &, uint16_t packet_id, int error_code)
        {
            on_unsubscribe(client->get_interface(), packet_id, error_code);
        }
    );
}

/**
 * Publishes to a topic.
 *
 * @param topic topic to publish to
 * @param qos QOS to publish the message with
 * @param retain should this message replace the current retained message of the topic?
 * @param payload payload of the message
 *
 * @return packet id of the publish request, or 0 if the attempt failed synchronously
 */
extern "C" uint16_t publish(InternalMqttClient *client, const char *topic, QOS qos, bool retain, Buffer data)
{
    auto connection = client->get_connection();
    return connection->Publish(
        topic, qos, retain, data.into(),
        [=](MqttConnection &, uint16_t packet_id, int error_code)
        {
            on_publish(client->get_interface(), packet_id, error_code);
        }
    );
}

extern "C" void disconnect(InternalMqttClient *client)
{
    client->get_connection()->Disconnect();
}

extern "C" void drop_client(InternalMqttClient *client)
{
    delete client;
}
