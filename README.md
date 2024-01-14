# Beluga

<p align="center">
  <img width="50%" height="auto" src="https://github.com/inomotech-foss/beluga/assets/3709476/7355924f-e54f-47e1-9832-78445dcaa80c">
</p>
<a href="https://www.vecteezy.com/free-vector/beluga">Image Source</a>

---
It's a Rust crate designed to serve as a wrapper for the official AWS IoT SDK.
It simplifies the integration of AWS IoT functionality into your Rust applications by providing a convenient and idiomatic Rust interface.

# TODO

(just tracking this here for the time being)

- use ci to check whether the bindings are correct (i.e. up-to-date and also they don't change depending on which platform they're generated on)
- use ci to run tests on various platforms to ensure we can compile on and to all the supported targets
- split up the beluga crate into parts basically like tokio and other crates do it (use feature flags to toggle features)
- use the rust allocator as the default for aws

# 🚀 Getting Started

## Install CMake

Ensure that you have cmake installed on your system. If not, you can download and install it from the official [CMake website](https://cmake.org/download/).

On Linux, you can typically install it using your package manager. For example, on Ubuntu:

```sh
sudo apt-get install cmake
```

On macOS, you can use Homebrew:

```sh
brew install cmake
```

Feel free to contribute to the project! Report issues, suggest new features, or contribute improvements to the project.
