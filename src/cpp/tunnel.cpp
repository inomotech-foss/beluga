#include <aws/iotsecuretunneling/IotSecureTunnelingClient.h>
#include <aws/iotsecuretunneling/SecureTunnel.h>
#include <aws/iotsecuretunneling/SecureTunnelingNotifyResponse.h>
#include <aws/iotsecuretunneling/SubscribeToTunnelsNotifyRequest.h>
#include <memory>
#include <vector>
#include "mqtt.h"
#include "logs.h"

namespace tunneling = Aws::Iotsecuretunneling;
namespace crt = Aws::Crt;

// tunnel client
extern "C" void on_subscribe_complete(const void *, int);
extern "C" void on_subscribe_tunnel(const void *, const char *, const char *, const char *);

// tunnel
/**
 * Type signature of the callback invoked when connection is established with the secure tunnel service and
 * available service ids are returned.
 */
extern "C" void on_connection_success(const void *, Buffer, Buffer, Buffer);
/**
 * Type signature of the callback invoked when connection is established with the secure tunnel service and
 * available service ids are returned.
 */
extern "C" void on_connection_failure(const void *, int);
/**
 * Type signature of the callback invoked when connection is shutdown.
 */
extern "C" void on_connection_shutdown(const void *);
/**
 * Type signature of the callback invoked when a connection has been reset
 */
extern "C" void on_connection_reset(const void *, int, uint32_t, Buffer);
/**
 * Type signature of the callback invoked when the secure tunnel receives a Session Reset.
 */
extern "C" void on_session_reset(const void *);
/**
 * Type signature of the callback invoked when message has been sent through the secure tunnel connection.
 */
extern "C" void on_send_message_complete(const void *, int, Buffer);
/**
 * Type signature of the callback invoked when a message is received through the secure tunnel connection.
 */
extern "C" void on_message_received(const void *, uint32_t, Buffer, Buffer);
/**
 * Type signature of the callback invoked when a stream has been started with a source through the secure tunnel
 * connection.
 */
extern "C" void on_stream_started(const void *, int, uint32_t, Buffer);
/**
 * Type signature of the callback invoked when a stream has been closed
 */
extern "C" void on_stream_stopped(const void *, Buffer);

std::function<void(tunneling::SecureTunnelingNotifyResponse *, int)> subscribe_callback(const void *interface);
std::function<void(int)> subscribe_complete_callback(const void *interface);
tunneling::OnConnectionSuccess connection_success(const void *interface);
tunneling::OnConnectionFailure connection_failure(const void *interface);
tunneling::OnConnectionShutdown connection_shutdown(const void *interface);
tunneling::OnConnectionReset connection_reset(const void *interface);
tunneling::OnSessionReset session_reset(const void *interface);
tunneling::OnSendMessageComplete send_message_complete(const void *interface);
tunneling::OnMessageReceived message_received(const void *interface);
tunneling::OnStreamStarted stream_started(const void *interface);
tunneling::OnStreamStopped stream_stopped(const void *interface);

extern "C"
{
    class InternalTunnelClient final
    {
    private:
        [[maybe_unused]] std::unique_ptr<tunneling::IotSecureTunnelingClient> client;

    public:
        InternalTunnelClient(std::unique_ptr<tunneling::IotSecureTunnelingClient> client) : client(std::move(client))
        {
        }
    };

    class InternalTunnel final
    {
    private:
        [[maybe_unused]] std::shared_ptr<tunneling::SecureTunnel> tunnel;

    public:
        InternalTunnel(std::shared_ptr<tunneling::SecureTunnel> tunnel) : tunnel(tunnel)
        {
        }

        std::shared_ptr<tunneling::SecureTunnel> get_tunnel() const
        {
            return this->tunnel;
        }
    };

    InternalTunnelClient *internal_tunnel_client(InternalMqttClient *mqtt_client,
                                                 const void *interface,
                                                 QOS qos,
                                                 const char *thing_name)
    {
        auto request = tunneling::SubscribeToTunnelsNotifyRequest();
        request.ThingName = AwsString(thing_name);

        auto tunnel_client = std::make_unique<tunneling::IotSecureTunnelingClient>(mqtt_client->get_connection());

        // The `if` statement is checking if the `tunnel_client` object is null or if the underlying
        // pointer is null. If either condition is true, it means that the `tunnel_client` object was
        // not successfully created, so `nullptr` is returned to indicate the failure.
        if (!tunnel_client && !*tunnel_client)
        {
            return nullptr;
        }

        tunnel_client->SubscribeToTunnelsNotify(
            request, qos, subscribe_callback(interface),
            subscribe_complete_callback(interface)
        );

        return new InternalTunnelClient(std::move(tunnel_client));
    }

    InternalTunnel *internal_tunnel(const void *interface, const char *endpoint, const char *access_token)
    {
        auto builder = tunneling::SecureTunnelBuilder(
            Aws::Crt::ApiAllocator(),
            access_token,
            AWS_SECURE_TUNNELING_DESTINATION_MODE,
            endpoint
        );

        builder.WithOnConnectionSuccess(connection_success(interface));
        builder.WithOnConnectionFailure(connection_failure(interface));
        builder.WithOnConnectionShutdown(connection_shutdown(interface));
        builder.WithOnConnectionReset(connection_reset(interface));
        builder.WithOnSessionReset(session_reset(interface));
        builder.WithOnSendMessageComplete(send_message_complete(interface));
        builder.WithOnMessageReceived(message_received(interface));
        builder.WithOnStreamStarted(stream_started(interface));
        builder.WithOnStreamStopped(stream_stopped(interface));

        auto tunnel = builder.Build();
        if (!tunnel)
        {
            error("tunnel equals to null");
            return nullptr;
        }

        return new InternalTunnel(tunnel);
    }

    /**
     * Deletes the object pointed to by the "tunnel_client" pointer.
     */
    void drop_internal_tunnel_client(InternalTunnelClient *tunnel_client)
    {
        delete tunnel_client;
    }

    /**
     * The function "drop_internal_tunnel" deletes a tunnel object.
     */
    void drop_internal_tunnel(InternalTunnel *tunnel)
    {
        delete tunnel;
    }

    /**
     * The function "start" calls the "Start" method of the tunnel object and returns the result.
     *
     * @param tunnel A pointer to an object of type "Tunnel".
     */
    int start(InternalTunnel *internal_tunnel)
    {
        auto tunnel = internal_tunnel->get_tunnel();
        return tunnel->Start();
    }

    /**
     * Stops the tunnel. This is equivalent to calling Stop () on the tunnel but does not wait for the stop to complete.
     *
     * @param internal_tunnel - * The tunnel to stop. Must be valid.
     *
     * @return 0 on success non - zero on failure. In this case the tunnel is stopped
     */
    int stop(InternalTunnel *internal_tunnel)
    {
        auto tunnel = internal_tunnel->get_tunnel();
        return tunnel->Stop();
    }

    /**
     * Sends a message through the tunnel.
     *
     * @param internal_tunnel The tunnel to send the message through.
     * @param connection_id The ID of the connection to send the message to.
     * @param payload The message payload.
     * @return An integer status code indicating success or failure.
     */
    int send_message(InternalTunnel *internal_tunnel, uint32_t connection_id, Buffer payload)
    {
        auto tunnel = internal_tunnel->get_tunnel();
        return tunnel->SendMessage(
            std::make_shared<tunneling::Message>(
                crt::ByteCursorFromByteBuf(payload.into()),
                connection_id
            )
        );
    }
}

Buffer buffer(crt::Optional<crt::ByteCursor> id)
{
    if (id)
    {
        return Buffer(id.value());
    }
    else
    {
        return Buffer();
    }
}

tunneling::OnConnectionSuccess connection_success(const void *interface)
{
    return [=](tunneling::SecureTunnel *, const tunneling::ConnectionSuccessEventData &data)
    {
        on_connection_success(
            interface,
            buffer(data.connectionData->getServiceId1()),
            buffer(data.connectionData->getServiceId2()),
            buffer(data.connectionData->getServiceId3())
        );
    };
}

tunneling::OnConnectionFailure connection_failure(const void *interface)
{
    return [=](tunneling::SecureTunnel *, int error_code)
    {
        on_connection_failure(
            interface,
            error_code
        );
    };
}

tunneling::OnConnectionShutdown connection_shutdown(const void *interface)
{
    return [=]()
    {
        on_connection_shutdown(interface);
    };
}

tunneling::OnConnectionReset connection_reset(const void *interface)
{
    return [=](tunneling::SecureTunnel *, int error_code, const tunneling::ConnectionResetEventData &data)
    {
        on_connection_reset(
            interface,
            error_code,
            data.connectionResetData->getConnectionId(),
            buffer(data.connectionResetData->getServiceId())
        );
    };
}

tunneling::OnSessionReset session_reset(const void *interface)
{
    return [=]()
    {
        on_session_reset(interface);
    };
}

tunneling::OnSendMessageComplete send_message_complete(const void *interface)
{
    return [=](tunneling::SecureTunnel *, int error_code, const tunneling::SendMessageCompleteEventData &data)
    {
        on_send_message_complete(
            interface,
            error_code,
            Buffer(data.sendMessageCompleteData->getMessageType())
        );
    };
}

tunneling::OnMessageReceived message_received(const void *interface)
{
    return [=](tunneling::SecureTunnel *, const tunneling::MessageReceivedEventData &data)
    {
        on_message_received(
            interface,
            data.message->getConnectionId(),
            buffer(data.message->getPayload()),
            buffer(data.message->getServiceId())
        );
    };
}

tunneling::OnStreamStarted stream_started(const void *interface)
{
    return [=](tunneling::SecureTunnel *, int error_code, const tunneling::StreamStartedEventData &data)
    {
        on_stream_started(
            interface,
            error_code,
            data.streamStartedData->getConnectionId(),
            buffer(data.streamStartedData->getServiceId())
        );
    };
}

tunneling::OnStreamStopped stream_stopped(const void *interface)
{
    return [=](tunneling::SecureTunnel *, const tunneling::StreamStoppedEventData &data)
    {
        on_stream_stopped(
            interface,
            buffer(data.streamStoppedData->getServiceId())
        );
    };
}

std::function<void(tunneling::SecureTunnelingNotifyResponse *, int)> subscribe_callback(const void *interface)
{
    return [=](tunneling::SecureTunnelingNotifyResponse *resp, int io_error) mutable
    {
        // check all the possible errors
        if (io_error != 0)
        {
            error(format("subscribing failed, error_code [%d]", io_error).c_str());
            return;
        }

        if (!resp)
        {
            error("response equals nullptr");
            return;
        }

        if (!resp->ClientAccessToken)
        {
            error("missing the access token");
            return;
        }

        if (!resp->Region)
        {
            error("missing the region");
            return;
        }

        if (!resp->ClientMode)
        {
            error("missing the client mode");
            return;
        }

        on_subscribe_tunnel(
            interface, resp->ClientAccessToken.value().c_str(),
            resp->Region.value().c_str(), resp->ClientMode.value().c_str()
        );
    };
}

std::function<void(int)> subscribe_complete_callback(const void *interface)
{
    return [=](int error_code)
    {
        on_subscribe_complete(interface, error_code);
    };
}
