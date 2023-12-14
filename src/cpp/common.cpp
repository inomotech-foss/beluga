#include "common.h"

ByteBuf Buffer::into() const
{
    return Aws::Crt::ByteBufNewCopy(Aws::Crt::ApiAllocator(), this->data, this->len);
}

Buffer::Buffer(const ByteBuf &buff) : data(buff.buffer), len(buff.len)
{
}

Buffer::Buffer(const ByteCursor &buff) : data(buff.ptr), len(buff.len)
{
}

Buffer::Buffer() : data(nullptr), len(0)
{
}

extern "C" bool is_buffer_empty(Buffer buffer);

bool Buffer::is_empty() const
{
    return is_buffer_empty(*this);
}
