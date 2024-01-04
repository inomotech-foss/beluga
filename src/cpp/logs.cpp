#include <aws/common/logging.h>
#include "logs.h"

extern "C" void rust_log(aws_log_level level, const char *file, const char *target, uint32_t line, const char *msg);

void info(const char *const msg, const char *file, const char *target, const int line)
{
    rust_log(AWS_LL_INFO, file, target, line, msg);
}

void error(const char *const msg, const char *file, const char *target, const int line)
{
    rust_log(AWS_LL_ERROR, file, target, line, msg);
}

void debug(const char *const msg, const char *file, const char *target, const int line)
{
    rust_log(AWS_LL_DEBUG, file, target, line, msg);
}

void warn(const char *const msg, const char *file, const char *target, const int line)
{
    rust_log(AWS_LL_WARN, file, target, line, msg);
}

void trace(const char *const msg, const char *file, const char *target, const int line)
{
    rust_log(AWS_LL_TRACE, file, target, line, msg);
}

int log_function(aws_logger *, aws_log_level level, aws_log_subject_t subject, const char *format, ...)
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

    rust_log(level, "", std::to_string(subject).c_str(), 0, buff.get());
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
    static auto vtable = aws_logger_vtable {
        log_function,
        log_level,
        clean_up,
        set_log_level};
    return &vtable;
}

aws_logger *logger()
{
    static auto logger = aws_logger {
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
