#include <aws/crt/Api.h>

extern "C"
{
    Aws::Crt::ApiHandle *create_api_handle()
    {
        return new Aws::Crt::ApiHandle();
    }

    void drop_api_handle(Aws::Crt::ApiHandle *handle)
    {
        delete handle;
    }
}
