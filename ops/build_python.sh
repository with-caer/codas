#!/bin/sh
#
# Usage: ./ops/build_python.sh

# Prepare virtual environment.
if [ ! -d /path/to/directory ]; then
    python3 -m venv venv
    source ./venv/bin/activate
    pip3 install maturin
else
    source ./venv/bin/activate
fi

# Build the packages.
maturin build -m codas-cdylib/Cargo.toml --features=python