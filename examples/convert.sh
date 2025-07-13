#!/bin/bash
cargo run -- \
    --file $(dirname $0)/resource/project/MDK-ARM/uv.uvprojx,target \
    --removed-args=--omf_browse,--depend \
    --extra-args="--I/path/to/include" \
    --pattern '/mnt/${disk}' \
    -o $(dirname $0) \
    --without-sysroot
