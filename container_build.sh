#!/bin/bash

dpkg --add-architecture i386
apt-get update
apt-get install -y --no-install-recommends ca-certificates gcc libc6-dev wget gcc-multilib clang curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > ~/rustup_init
chmod +x ~/rustup_init
~/rustup_init -y --profile minimal -t i686-unknown-linux-gnu
. ~/.cargo/env
cargo build --release --target i686-unknown-linux-gnu