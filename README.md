# easyifconfig

`ifconfig` in plain language. It runs `ifconfig`, parses the output, and
explains each interface: what it is, whether it's connected, and what its
addresses mean.

Built for macOS. The interface labels (`utun`, `awdl`, `anpi`, `bridge`…)
follow macOS naming.

## Install

```sh
cargo install --path .
```

## Usage

```sh
easyifconfig
```

Sample output:

```
lo0 · Loopback
  Your machine talking to itself. Traffic never leaves it.
  ● up
  IPv4  127.0.0.1/8
        loopback: only this machine can reach it
  IPv6  ::1/128
        loopback
  MTU   16384
        largest packet in bytes; the usual is 1500
```

## Layout

- `src/ifconfig.rs` runs `ifconfig` and returns its output.
- `src/main.rs` splits the output into one block per interface.
- `src/block.rs` parses a block into an `Iface`.
- `src/knowledge.rs` maps names and addresses to plain-language hints.
- `src/render.rs` prints the friendly view.
- `samples/mac.txt` is captured `ifconfig` output for tests.

## Test

```sh
cargo test
```
