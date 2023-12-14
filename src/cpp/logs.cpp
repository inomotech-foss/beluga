#include <aws/common/logging.h>
#include "logs.h"

extern "C" void rust_info(const char *const file, const char *const name, const int line, const char *const msg);
extern "C" void rust_error(const char *const file, const char *const name, const int line, const char *const msg);
extern "C" void rust_debug(const char *const file, const char *const name, const int line, const char *const msg);
extern "C" void rust_warn(const char *const file, const char *const name, const int line, const char *const msg);
extern "C" void rust_trace(const char *const file, const char *const name, const int line, const char *const msg);

void info(const char *const msg, const char *file, const char *name, const int line)
{
    rust_info(file, name, line, msg);
}

void error(const char *const msg, const char *file, const char *name, const int line)
{
    rust_error(file, name, line, msg);
}

void debug(const char *const msg, const char *file, const char *name, const int line)
{
    rust_debug(file, name, line, msg);
}

void warn(const char *const msg, const char *file, const char *name, const int line)
{
    rust_warn(file, name, line, msg);
}

void trace(const char *const msg, const char *file, const char *name, const int line)
{
    rust_trace(file, name, line, msg);
}

int log_function(aws_logger *, aws_log_level level, aws_log_subject_t, const char *format, ...)
{
    std::va_list args;
    va_start(args, format);
    std::va_list args_cp;
    va_copy(args_cp, args);
    auto size = static_cast<size_t>(1 + std::vsnprintf(nullptr, 0, format, args));
    auto buff = std::make_unique<char[]>(size);
    va_end(args);
    std::vsnprintf(buff.get(), size, format, args_cp);
    va_end(args_cp);

    switch (level)
    {
    case AWS_LOG_LEVEL_NONE:
    case AWS_LOG_LEVEL_TRACE:
        rust_trace("", "", 0, buff.get());
        break;
    case AWS_LOG_LEVEL_DEBUG:
        rust_debug("", "", 0, buff.get());
        break;
    case AWS_LOG_LEVEL_WARN:
        rust_warn("", "", 0, buff.get());
        break;
    case AWS_LOG_LEVEL_FATAL:
    case AWS_LOG_LEVEL_ERROR:
        rust_error("", "", 0, buff.get());
        break;
    case AWS_LOG_LEVEL_INFO:
        rust_info("", "", 0, buff.get());
        break;
    default:
        return AWS_OP_ERR;
    }

    return AWS_OP_SUCCESS;
}

aws_log_level log_level(aws_logger *, aws_log_subject_t)
{
    return aws_log_level::AWS_LL_TRACE;
}

void clean_up(aws_logger *)
{
    // function does nothing.
}

int set_log_level(aws_logger *, aws_log_level)
{
    return AWS_OP_SUCCESS;
}

aws_logger_vtable *logger_vtable()
{
    static auto vtable = aws_logger_vtable{
        log_function,
        log_level,
        clean_up,
        set_log_level
    };
    return &vtable;
}

aws_logger *logger()
{
    static auto logger = aws_logger{
        logger_vtable(),
        aws_default_allocator(),
        nullptr,
    };
    return &logger;
}

void init_logger()
{
    aws_logger_set(logger());
}
