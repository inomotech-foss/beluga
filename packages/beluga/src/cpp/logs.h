#pragma once
#include <cstdarg>
#include <memory>

extern "C" void info(const char *const msg);
extern "C" void debug(const char *const msg);
extern "C" void error(const char *const msg);

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
    std::unique_ptr<char[]> buf(new char[size]);
    std::snprintf(buf.get(), size, format.c_str(), args...);
    return std::string(buf.get(), buf.get() + size - 1); // We don't want the '\0' inside
}
