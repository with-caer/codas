FROM fedora:43

# Common development packages.
RUN dnf install -y awk git gcc g++ cmake

# Rust toolchain.
RUN dnf install -y rustup
RUN rustup-init -y
ENV PATH="/root/.cargo/bin:${PATH}"