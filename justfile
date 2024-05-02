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

circt_sys_dir := justfile_directory() / "vts_circt" / "circt-sys"

format-cpp:
    clang-format -i {{circt_sys_dir / "wrapper.h"}} {{circt_sys_dir / "wrapper.cpp"}}

circt_dir := circt_sys_dir / "circt"
llvm_dir := circt_dir / "llvm"
circt_build_dir := circt_dir / "build"
llvm_build_dir := llvm_dir / "build"

build-llvm:
    mkdir -p {{llvm_build_dir}}
    cmake -S {{llvm_dir / "llvm"}} -B {{llvm_build_dir}} -G Ninja \
        -DLLVM_ENABLE_PROJECTS="mlir" \
        -DLLVM_TARGETS_TO_BUILD="host" \
        -DLLVM_ENABLE_ASSERTIONS=ON \
        -DCMAKE_BUILD_TYPE=RelWithDebInfo \
        -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
    cmake --build {{llvm_build_dir}}

clean-llvm:
    rm -r {{llvm_build_dir}}

build-circt:
    mkdir -p {{circt_build_dir}}
    cmake -S {{circt_dir}} -B {{circt_build_dir}} -G Ninja \
        -DMLIR_DIR={{llvm_build_dir / "lib" / "cmake" / "mlir"}} \
        -DLLVM_DIR={{llvm_build_dir / "lib" / "cmake" / "llvm"}} \
        -DLLVM_ENABLE_ASSERTIONS=ON \
        -DCMAKE_BUILD_TYPE=RelWithDebInfo \
        -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
    cmake --build {{circt_build_dir}}

clean-circt:
    rm -r {{circt_build_dir}}
