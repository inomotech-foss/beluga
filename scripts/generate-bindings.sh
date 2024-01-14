#!/usr/bin/env bash

# TODO: replace with rust impl

main() {
    # these targets are chosen somewhat arbitrarily. we need a better structure
    local targets=(
        "aarch64-apple-darwin"
        "armv7-unknown-linux-musleabihf"
        "x86_64-pc-windows-gnu"
    )

    for target in "${targets[@]}"; do
        echo "target: $target"
        cargo zigbuild -q --target "$target" --workspace --features aws-c-builder/generate-bindings
    done
}

main
