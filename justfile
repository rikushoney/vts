install:
    pip install -r requirements/dev.txt

format-rs:
    cargo fmt --all

format-py:
    ruff check --select I --fix python/vts
    black --quiet python/vts

format: format-rs format-py

lock:
    pip-compile -o requirements/tests.txt requirements/tests.in 
    pip-compile -o requirements/check.txt requirements/check.in 
    pip-compile -o requirements/format.txt requirements/format.in 
    pip-compile -o requirements/build.txt requirements/build.in 
    pip-compile -o requirements/dev.txt requirements/dev.in

check-rs:
    cargo check --workspace
    cargo clippy --workspace

check-py:
    nox -s check

check-all: check-rs check-py

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

clean:
    cargo clean
