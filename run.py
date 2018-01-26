#!/usr/bin/env python

import sys
import subprocess

argvs = sys.argv
argc = len(argvs)

if argc >= 2  and argv[1] == "debug":
    cmd = "env RUST_BACKTRACE=1 cargo run -- --debug debug.txt --level trace --vis --maxloop 1000"
else:
    cmd = "cargo run --release -- --vis --maxloop 1000"

subprocess.call(cmd.strip().split(" ")) 
