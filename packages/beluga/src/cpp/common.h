#pragma once
#include <cstddef>
#include <aws/crt/Types.h>

using ByteBuf = Aws::Crt::ByteBuf;
using ByteCursor = Aws::Crt::ByteCursor;
using QOS = Aws::Crt::Mqtt::QOS;
using ReturnCode = Aws::Crt::Mqtt::ReturnCode;
using AwsString = Aws::Crt::String;

extern "C"
{
    struct Buffer
    {
        uint8_t *data;
        size_t len;

        Buffer(const ByteBuf &buff);
        Buffer(const ByteCursor &buff);
        Buffer();

        ByteBuf into() const;
        bool is_empty() const;
    };
}
