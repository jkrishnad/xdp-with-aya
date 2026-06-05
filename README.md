# xdp

An aya-based XDP program. Built and run on a remote Linux box over SSH (the
local machine is macOS, which can't load XDP), orchestrated by the `xtask`.

## Prerequisites (local / macOS)

- Rust stable + the `xtask` workspace member (already in this repo)
- An SSH key that can reach the remote box
- `rsync` and `ssh` (preinstalled on macOS)

## Remote box

A Linux server (Ubuntu 24.04 tested) reachable over SSH. Connection details
are set as constants at the top of `xtask/src/main.rs`:

- `SSH_HOST` — e.g. `root@<ip>`
- `SSH_KEY` — absolute path to your private key
- `LOCAL_DIR` / `REMOTE_DIR` — code paths on each side

## Build & Run

One-time, installs the toolchain (rust nightly + rust-src + bpf-linker) on
the remote.

```
cd xtask

cargo xtask setup
```

Then sync + build + run, attaching the XDP program to an interface:

```
cargo xtask run -i lo
```

`xtask` rsyncs the code to the remote, builds it there (the build script
compiles the eBPF automatically), and runs the program on the chosen
interface. Ctrl-C to stop.

Other tasks: `cargo xtask build`, `cargo xtask check`, `cargo xtask sync`,
`cargo xtask ssh` (interactive shell on the remote).

## License

Dual MIT / Apache-2.0.
