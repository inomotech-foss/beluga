#include <aws/common/logging.h>
#include <stdarg.h>

// see logging.rs for documentation

struct beluga_logger {
  void *p_impl;
  bool (*log_enabled)(void *p_impl, enum aws_log_level log_level,
                      aws_log_subject_t subject);
  int (*log)(void *p_impl, enum aws_log_level log_level,
             aws_log_subject_t subject, struct aws_string *message);
};

static struct aws_string *s_format_to_string(struct aws_allocator *allocator,
                                             const char *format, va_list args) {
  va_list tmp_args;
  va_copy(tmp_args, args);
#ifdef _MSC_VER
  int required_length = _vscprintf(format, tmp_args) + 1;
#else
  int required_length = vsnprintf(NULL, 0, format, tmp_args) + 1;
#endif
  va_end(tmp_args);

  struct aws_string *raw_string =
      aws_mem_calloc(allocator, 1, sizeof(struct aws_string) + required_length);
  if (raw_string == NULL) {
    return NULL;
  }
  *(struct aws_allocator **)(&raw_string->allocator) = allocator;

#ifdef _MSC_VER
  int written_count = vsnprintf_s((char *)raw_string->bytes, required_length,
                                  _TRUNCATE, format, args);
#else
  int written_count =
      vsnprintf((char *)raw_string->bytes, required_length, format, args);
#endif /* _MSC_VER */
  if (written_count < 0) {
    aws_string_destroy(raw_string);
    return NULL;
  }

  *(size_t *)(&raw_string->len) = written_count;

  return raw_string;
}

int beluga_logging_log(struct aws_logger *logger, enum aws_log_level log_level,
                       aws_log_subject_t subject, const char *format, ...) {
  struct beluga_logger *b_log = (struct beluga_logger *)logger->p_impl;
  if (!b_log->log_enabled(b_log->p_impl, log_level, subject)) {
    return AWS_OP_SUCCESS;
  }

  va_list args;
  va_start(args, format);
  struct aws_string *message =
      s_format_to_string(logger->allocator, format, args);
  va_end(args);

  if (message == NULL) {
    return AWS_OP_ERR;
  }

  return b_log->log(b_log->p_impl, log_level, subject, message);
}
