install:
    pip install -r requirements/dev.txt

format-rs:
    cargo fmt --all

format-py:
    ruff check --select I --fix python/vts scratchpad.ipynb
    black --quiet python/vts scratchpad.ipynb

format: format-rs format-py

lock:
    pip-compile --strip-extras -o requirements/tests.txt requirements/tests.in 
    pip-compile --strip-extras -o requirements/check.txt requirements/check.in 
    pip-compile --strip-extras -o requirements/format.txt requirements/format.in 
    pip-compile --strip-extras -o requirements/build.txt requirements/build.in 
    pip-compile --strip-extras -o requirements/notebook.txt requirements/notebook.in
    pip-compile --strip-extras -o requirements/dev.txt requirements/dev.in

check-rs:
    cargo check --workspace
    cargo clippy --workspace

check-py:
    nox -s check
    nbqa ruff scratchpad.ipynb --ignore E402
    nbqa mypy scratchpad.ipynb

check-md:
    markdownlint-cli2 README.md

check-all: check-rs check-py check-md

check CRATE:
    cargo check --package vts_{{CRATE}}
    cargo clippy --package vts_{{CRATE}}

test-rs:
    cargo test --workspace --exclude vts_api

test-py:
    nox -s tests

test-all: test-rs test-py

test CRATE:
    cargo test --package vts_{{CRATE}}

build-rs:
    cargo build --workspace --exclude vts_api

build-py:
    maturin build

build: build-rs build-py

notebook:
    jupyter-lab

clean:
    cargo clean

watch-cargo:
    cargo watch -c -w vts_api -w vts_core -w vts_cli -x check -x clippy

watch-maturin:
    watchexec -c -w vts_api -w vts_core maturin dev

watch-git:
    watchexec -c -r -w ./ -w .git/objects git status

watch-scratchpad:
    watchexec -c -r RUST_BACKTRACE=1 python scratchpad.py
