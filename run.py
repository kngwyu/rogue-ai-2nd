#!/usr/bin/env python

import sys
import subprocess

argvs = sys.argv
argc = len(argvs)

if argc >= 2  and (argvs[1] == "--debug" or argvs[1] == "-D"):
    cmd = "env RUST_BACKTRACE=1 cargo run -- --debug debug.txt --level trace --vis --maxloop 3000 --interval 70"
else:
    cmd = "cargo run --release -- --vis --maxloop 3000 --interval 70"

subprocess.call(cmd.strip().split(" ")) 
