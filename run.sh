#!/usr/bin/sh
env RUST_BACKTRACE=1 cargo run -- --debug debug.txt --level trace --vis --maxloop 1000
