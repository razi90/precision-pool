#!/bin/sh
scrypto build
wasm-opt -Os -g --strip-debug --strip-dwarf --strip-producers -o target/wasm32-unknown-unknown/release/ociswap_hooks_opt.wasm target/wasm32-unknown-unknown/release/ociswap_hooks.wasm
ls -l target/wasm32-unknown-unknown/release/ociswap_hooks_opt.wasm