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
    struct Buffer final
    {
        uint8_t *data;
        size_t len;
        bool owned;

        explicit Buffer() noexcept;
        explicit Buffer(const ByteBuf &buff) noexcept;
        explicit Buffer(const ByteCursor &buff) noexcept;
        Buffer(const Buffer &) = delete;
        Buffer(Buffer &&other) noexcept;

        Buffer &operator=(Buffer &&) noexcept;

        ByteBuf into() const;
        /**
         * Checks if this Buffer is empty (i.e. len == 0).
         */
        bool is_empty() const;
        /**
         * Returns whether this Buffer owns the underlying data buffer.
         */
        bool is_owned() const;

        static Buffer create(size_t size);

        ~Buffer();
    };
}
