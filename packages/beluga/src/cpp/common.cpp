#include "common.h"

ByteBuf Buffer::into() const
{
    return Aws::Crt::ByteBufNewCopy(Aws::Crt::ApiAllocator(), this->data, this->len);
}

Buffer::Buffer(const ByteBuf &buff) noexcept : data(buff.buffer), len(buff.len), owned(false)
{
}

Buffer::Buffer(const ByteCursor &buff) noexcept : data(buff.ptr), len(buff.len), owned(false)
{
}

Buffer::Buffer() noexcept : data(nullptr), len(0), owned(false)
{
}

Buffer::Buffer(Buffer &&other) noexcept
{
    *this = std::move(other);
}

Buffer &Buffer::operator=(Buffer &&other) noexcept
{
    if (this != &other)
    {
        this->data = other.data;
        this->len = other.len;
        this->owned = other.owned;
        other.data = nullptr;
        other.len = 0;
        other.owned = false;
    }
    return *this;
}

extern "C" bool is_buffer_empty(const Buffer *buffer);
extern "C" Buffer create_buffer(size_t size);
extern "C" bool destroy_buffer(Buffer *buffer);

Buffer Buffer::create(size_t size)
{
    return std::move(create_buffer(size));
}

bool Buffer::is_empty() const
{
    return is_buffer_empty(this);
}

bool Buffer::is_owned() const
{
    return this->owned;
}

Buffer::~Buffer()
{
    if (this->is_owned())
    {
        destroy_buffer(this);
        this->owned = false;
    }
}
