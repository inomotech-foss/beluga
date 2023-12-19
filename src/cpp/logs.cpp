#include <aws/common/logging.h>
#include "logs.h"

extern "C" void rust_log(aws_log_level level, aws_log_subject_t subject, aws_string *message);

void info(const char *const msg, const char *file, const char *name, const int line)
{
    rust_log(AWS_LL_INFO, 0, msg);
}

void error(const char *const msg, const char *file, const char *name, const int line)
{
    rust_log(AWS_LL_ERROR, 0, msg);
}

void debug(const char *const msg, const char *file, const char *name, const int line)
{
    rust_log(AWS_LL_DEBUG, 0, msg);
}

void warn(const char *const msg, const char *file, const char *name, const int line)
{
    rust_log(AWS_LL_WARN, 0, msg);
}

void trace(const char *const msg, const char *file, const char *name, const int line)
{
    rust_log(AWS_LL_TRACE, 0, msg);
}

int log_function(aws_logger *logger, aws_log_level level, aws_log_subject_t subject, const char *format, ...)
{
    va_list args;
    va_start(args, format);

    va_list tmp_args;
    va_copy(tmp_args, args);
    auto total_length = vsnprintf(nullptr, 0, format, tmp_args) + 1;
    va_end(tmp_args);

    struct aws_string *raw_string = aws_mem_calloc(logger->allocator, 1, sizeof(struct aws_string) + total_length);
    if (raw_string == NULL) {
        // TODO
    }

    int written_count = vsnprintf(
        raw_string->bytes,
        total_length,
        format,
        args);
    if (written_count < 0) {
        // TODO
    }

    va_end(args);

    rust_log(level, subject, raw_string);
    aws_string_destroy(raw_string);

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
