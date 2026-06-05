use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

// Connection config: this is how the xtask knows WHERE to go
const SSH_HOST: &str = "root@168.144.115.79"; // your droplet
const SSH_KEY: &str = "/Users/jayakrishna/.ssh/id_ed25519";
const LOCAL_DIR: &str = "/Users/jayakrishna/Documents/svm/xdp"; // code on your Mac
const REMOTE_DIR: &str = "/root/xdp"; // where it lands on the droplet

#[derive(Parser)]
#[command(
    name = "xtask",
    about = "Build & run the aya XDP program on a remote Linux box over SSH"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// One-time: install rust nightly + bpf-linker + build deps on the remote box.
    Setup,
    /// rsync code to the remote and build it there (eBPF built automatically by build script).
    Build,
    /// Sync + build + run the XDP program on the remote, attached to an interface. Ctrl-C to stop.
    Run {
        #[arg(short, long, default_value = "lo")]
        iface: String,
    },
    /// Run clippy on the remote.
    Check,
    /// Just push local code to the remote (no build).
    Sync,
    /// Open an interactive shell on the remote.
    Ssh,
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Setup => setup()?,
        Cmd::Sync => sync()?,
        Cmd::Build => {
            sync()?;
            remote(&format!("cd {REMOTE_DIR} && cargo build --release"))?;
        }
        Cmd::Run { iface } => {
            sync()?;
            // Build as root (it's a root droplet), then run the binary directly.
            // Root already has kernel privileges, so no separate sudo needed.
            remote(&format!(
                "cd {REMOTE_DIR} && cargo build --release -p xdp && \
                 RUST_LOG=info ./target/release/xdp -i {iface}"
            ))?;
        }
        Cmd::Check => {
            sync()?;
            remote(&format!("cd {REMOTE_DIR} && cargo clippy --all-targets"))?;
        }
        Cmd::Ssh => {
            // Interactive: hand the terminal over to ssh.
            run("ssh", &["-i", SSH_KEY, "-t", SSH_HOST])?;
        }
    }
    Ok(())
}

/// Install the eBPF toolchain on the remote box (idempotent).
fn setup() -> Result<()> {
    eprintln!("[xtask] Installing toolchain on {SSH_HOST} (first run is slow)...");
    remote(
        "set -eux; \
         export DEBIAN_FRONTEND=noninteractive; \
         apt-get update; \
         apt-get install -y build-essential pkg-config libelf-dev clang llvm curl rsync; \
         if ! command -v rustup >/dev/null; then \
           curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
         fi; \
         source \"$HOME/.cargo/env\"; \
         rustup toolchain install stable; \
         rustup toolchain install nightly --component rust-src; \
         command -v bpf-linker >/dev/null || cargo install bpf-linker",
    )
    .context("remote toolchain install failed")?;
    eprintln!("[xtask] Setup complete. Try: cargo xtask run -i lo");
    Ok(())
}

/// Push local code to the remote, excluding build artifacts and git.
fn sync() -> Result<()> {
    eprintln!("[xtask] Syncing {LOCAL_DIR} -> {SSH_HOST}:{REMOTE_DIR}");
    run(
        "rsync",
        &[
            "-az",
            "--delete",
            "--exclude",
            "target/",
            "--exclude",
            ".git/",
            "-e",
            &format!("ssh -i {SSH_KEY}"),
            // trailing slash on source = copy contents into REMOTE_DIR
            &format!("{LOCAL_DIR}/"),
            &format!("{SSH_HOST}:{REMOTE_DIR}/"),
        ],
    )
    .context("rsync to remote failed")
}

/// Run a command on the remote over SSH. We pass the whole thing as ONE argument
/// wrapped in `bash -lc '...'` so SSH hands it to a login shell intact (cargo on PATH).
fn remote(script: &str) -> Result<()> {
    // Single-quote the script for the remote shell, escaping any embedded single quotes.
    let quoted = format!("bash -lc '{}'", script.replace('\'', r"'\''"));
    run("ssh", &["-i", SSH_KEY, "-t", SSH_HOST, &quoted])
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    eprintln!("[xtask] $ {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to spawn `{program}` (installed and on PATH?)"))?;
    if !status.success() {
        match status.code() {
            Some(code) => bail!("`{program}` exited with status {code}"),
            None => bail!("`{program}` was terminated by a signal"),
        }
    }
    Ok(())
}
