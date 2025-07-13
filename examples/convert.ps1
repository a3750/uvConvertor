#!/usr/bin/env pwsh
cargo run -- `
    --file "$PSScriptRoot/resource/project/MDK-ARM/uv.uvprojx,target" `
    --removed-args=--omf_browse,--depend `
    --extra-args="--I/path/to/include" `
    --pattern '/mnt/${disk}' `
    -o "$PSScriptRoot" `
    --without-sysroot
