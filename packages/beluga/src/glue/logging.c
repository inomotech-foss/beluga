#include <aws/common/logging.h>
#include <stdarg.h>

// naming:
//  acglu: AWS CRT Glue
//  arglu: AWS Rust Glue

bool arglu_log_enabled(void *p_impl, enum aws_log_level log_level,
                       aws_log_subject_t subject);
int arglu_log(void *p_impl, enum aws_log_level log_level,
              aws_log_subject_t subject, struct aws_string *message);

static struct aws_string *
s_acglu_format_to_string(struct aws_allocator *allocator, const char *format,
                         va_list args) {
  va_list tmp_args;
  va_copy(tmp_args, args);
#ifdef _WIN32
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

#ifdef _WIN32
  int written_count = vsnprintf_s((char *)raw_string->bytes, required_length,
                                  _TRUNCATE, format, args);
#else
  int written_count =
      vsnprintf((char *)raw_string->bytes, required_length, format, args);
#endif /* _WIN32 */
  if (written_count < 0) {
    aws_string_destroy(raw_string);
    return NULL;
  }

  *(size_t *)(&raw_string->len) = written_count;

  return raw_string;
}

int acglu_log(struct aws_logger *logger, enum aws_log_level log_level,
              aws_log_subject_t subject, const char *format, ...) {
  if (!arglu_log_enabled(logger->p_impl, log_level, subject)) {
    return AWS_OP_SUCCESS;
  }

  va_list args;
  va_start(args, format);
  struct aws_string *message =
      s_acglu_format_to_string(logger->allocator, format, args);
  va_end(args);

  if (message == NULL) {
    return AWS_OP_ERR;
  }

  return arglu_log(logger->p_impl, log_level, subject, message);
}
