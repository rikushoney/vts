FROM quay.io/pypa/manylinux_2_28_x86_64 AS base

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN pipx install maturin

FROM base AS final

ENV PATH /root/.cargo/bin:$PATH
WORKDIR /home/workdir
CMD ["maturin", "build", "--release"]
