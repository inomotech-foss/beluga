#pragma once
#include <aws/crt/Api.h>
#include <aws/iot/MqttClient.h>
#include <aws/crt/mqtt/MqttConnection.h>
#include "common.h"

using MqttBuilder = Aws::Iot::MqttClientConnectionConfigBuilder;
using MqttConfig = Aws::Iot::MqttClientConnectionConfig;
using MqttConnection = Aws::Crt::Mqtt::MqttConnection;

extern "C"
{
    // It's more for mental health than for a real usage through FFI
    class InternalMqttClient final
    {
    private:
        std::shared_ptr<MqttConnection> connection;
        const void *interface;

    public:
        InternalMqttClient(std::shared_ptr<MqttConnection> connection, const void *interface);
        std::shared_ptr<MqttConnection> get_connection() const;
        const void *get_interface() const;
    };

    struct ClientConfig
    {
        const char *endpoint;
        uint16_t port;
        const char *client_id;
        bool clean_session;
        uint16_t keep_alive_s;
        uint32_t ping_timeout_ms;
        const char *username;
        const char *password;
        Buffer certificate;
        Buffer private_key;
    };
}
