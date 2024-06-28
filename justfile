install:
    pip install -r requirements/dev.txt

format-rs:
    cargo fmt --all

format-py:
    ruff check --select I --fix python/vts scratchpad.ipynb
    black --quiet python/vts scratchpad.ipynb

format: format-rs format-py

lock:
    pip-compile -U --strip-extras -o requirements/tests.txt requirements/tests.in 
    pip-compile -U --strip-extras -o requirements/check.txt requirements/check.in 
    pip-compile -U --strip-extras -o requirements/format.txt requirements/format.in 
    pip-compile -U --strip-extras -o requirements/build.txt requirements/build.in 
    pip-compile -U --strip-extras -o requirements/notebook.txt requirements/notebook.in
    pip-compile -U --strip-extras -o requirements/dev.txt requirements/dev.in

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

circt_sys_dir := justfile_directory() / "vts_circt" / "circt-sys"

format-cpp:
    clang-format -i {{circt_sys_dir / "wrapper.h"}} {{circt_sys_dir / "wrapper.cpp"}}

circt_src_dir := circt_sys_dir / "circt"
circt_build_dir := circt_sys_dir / "build"
circt_install_dir := circt_sys_dir / "install"

build-circt:
    cmake -G Ninja -B {{circt_build_dir}} -S {{circt_src_dir / "llvm" / "llvm"}} \
        -DCMAKE_BUILD_TYPE=RelWithDebInfo \
        -DCMAKE_INSTALL_PREFIX={{circt_install_dir}} \
        -DLLVM_ENABLE_PROJECTS=mlir \
        -DLLVM_ENABLE_ASSERTIONS=ON \
        -DLLVM_ENABLE_ZSTD=OFF \
        -DLLVM_EXTERNAL_PROJECTS=circt \
        -DLLVM_EXTERNAL_CIRCT_SOURCE_DIR={{circt_src_dir}} \
        -DLLVM_TARGETS_TO_BUILD=host
    cmake --build {{circt_build_dir}}

install-circt:
    cmake --install {{circt_build_dir}}

clean-circt-build:
    rm -r {{circt_build_dir}}

clean-circt: clean-circt-build
    rm -r {{circt_install_dir}}

generate-bindings:
    bindgen -o {{circt_sys_dir / "src" / "bindings.rs"}} {{circt_sys_dir / "wrapper.h"}} -- \
        -I$({{circt_install_dir / "bin" / "llvm-config"}} --includedir)
