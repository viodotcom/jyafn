# Installation

!!! failure "JYAFN is not available in Windows"
    JYAFN is currently not available for Windows targets. If you want to run JYAFN in a Windows machine, we recommend you use WSL or aa virtual machine.

## Installing the Python package

The Python package for `jyafn` is available in PyPI and is easily downloadable with this simple command:
```sh
pip install jyafn
```
!!! note
    The minimum Python version that JYAFN supports is Python 3.10

This also installs the `jyafn` CLI tool.

## Getting the dynamic library

For integration with production environments other than Rust or Python, the recommended way to use JYAFN is through the dynamic library that is made available in GitHub. You can get the latest version of the library directly from Github Releases, like so:

=== "Linux"
    ```sh
    curl -L \
        https://github.com/viodotcom/jyafn/releases/latest/download/libcjyafn.so \
        --output /usr/local/lib/libcjyafn.so
    ```
=== "macOS"
    ```sh
    curl -L \
        https://github.com/viodotcom/jyafn/releases/latest/download/libcjyafn.dylib \
        --output /usr/local/lib/libcjyafn.dylib
    ```

    !!! info "Only ARM targets supported"
        Only ARM platforms are supported for macOS at the moment. This might change in the future, if there is demand for x64 machines.

Since dynamic libraries are installed by default to `/usr/local/lib` in Unix-like systems, you might need `sudo` to perform this operation. If this is not possible, a solution is to install it locally and tweak the `LD_LIBRARY_PATH` environment variable.

## Using JYAFN in Rust projects

JYAFN is also available in [crates.io](https://crates.io/crates/jyafn). You can add the `jyafn` package to your project using `cargo`:
```sh
cargo add jyafn
```
Note that you will need to have `gcc` to buid this crate.
