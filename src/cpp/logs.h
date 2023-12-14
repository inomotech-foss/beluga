#pragma once
#include <cstdarg>
#include <string>
#include <memory>

void info (const char *const msg, const char *file = __builtin_FILE(), const char *name = __builtin_FUNCTION(), const int line = __builtin_LINE());
void error(const char *const msg, const char *file = __builtin_FILE(), const char *name = __builtin_FUNCTION(), const int line = __builtin_LINE());
void debug(const char *const msg, const char *file = __builtin_FILE(), const char *name = __builtin_FUNCTION(), const int line = __builtin_LINE());
void warn (const char *const msg, const char *file = __builtin_FILE(), const char *name = __builtin_FUNCTION(), const int line = __builtin_LINE());
void trace(const char *const msg, const char *file = __builtin_FILE(), const char *name = __builtin_FUNCTION(), const int line = __builtin_LINE());

extern "C"
{
    void init_logger();
}

/**
 * The function is a variadic template function that takes a format string and a variable number of arguments.
 *
 * @param format format string
 * @param args arguments that will be formatted
 *
 * @return formatted `std::string`
 */
template <typename... Args>
std::string format(const std::string &format, Args... args)
{
    int size_s = std::snprintf(nullptr, 0, format.c_str(), args...) + 1; // Extra space for '\0'
    if (size_s <= 0)
    {
        return std::string();
    }
    auto size = static_cast<size_t>(size_s);
    auto buff = std::make_unique<char []>(size);
    std::snprintf(buff.get(), size, format.c_str(), args...);
    return std::string(buff.get(), buff.get() + size - 1); // We don't want the '\0' inside
}
