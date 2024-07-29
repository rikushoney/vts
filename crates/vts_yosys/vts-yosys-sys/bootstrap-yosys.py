#!/usr/bin/env python

import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Iterator, NamedTuple

VTS_YOSYS_SYS_DIR: Path = Path(__file__).parent
YOSYS_ROOT_DIR: Path = VTS_YOSYS_SYS_DIR / "yosys"
YOSYS_MAKEFILE_INC: str = "Makefile.inc"
YOSYS_BASE_MODULES: set[str] = {"backends", "frontends", "libs", "passes"}
YOSYS_BLACKLISTED_MODULES: set[str] = {"passes/pmgen"}
YOSYS_BLACKLISTED_SOURCES: set[str] = {
    "kernel/driver.cc",
    "frontends/rtlil/rtlil_lexer.cc",
    "frontends/rtlil/rtlil_parser.tab.cc",
    "frontends/verilog/verilog_lexer.cc",
    "frontends/verilog/verilog_parser.tab.cc",
    "passes/techmap/abc.cc",
    "passes/techmap/abc9.cc",
    "passes/techmap/abc9_exe.cc",
    "passes/techmap/abc9_ops.cc",
}
YOSYS_BUILD_INCLUDE_DIR: Path = VTS_YOSYS_SYS_DIR / "include"
YOSYS_GENERATE_CELLHELP: set[str] = {"simlib.v", "simcells.v"}
YOSYS_TECHLIBS_COMMON: Path = YOSYS_ROOT_DIR / "techlibs" / "common"
YOSYS_CELLHELP_PY: Path = YOSYS_TECHLIBS_COMMON / "cellhelp.py"


def eprint(*args: Any, **kwargs: Any) -> None:
    kwargs["file"] = sys.stderr
    print(*args, **kwargs)


def iter_kernel_sources() -> Iterator[str]:
    def isvalid(path: Path) -> bool:
        return path.suffix == ".cc"

    def getrel(path: Path) -> str:
        return path.relative_to(YOSYS_ROOT_DIR).as_posix()

    yield from map(getrel, filter(isvalid, (YOSYS_ROOT_DIR / "kernel").iterdir()))


def process_line(line: str) -> Iterator[str]:
    def isvalid(entry: str) -> bool:
        return len(entry) > 0

    def cleanup(entry: str) -> str:
        return entry.strip().removesuffix(".o") + ".cc"

    yield from filter(isvalid, map(cleanup, line.split()))


def _parse_makefile_inc(makefile_inc: Path) -> Iterator[str]:
    for i, line in enumerate(makefile_inc.read_text().splitlines()):
        line = line.strip()
        if line.startswith("OBJS"):
            needle = "+="
            jump = line.find(needle) + len(needle)
            if jump < len(needle):
                raise ValueError(f'expected "{needle}" on line {i + 1}:"{line}"')
            line = line[jump:].lstrip()
            yield from process_line(line)


def parse_makefile_inc(makefile_inc: Path) -> Iterator[str]:
    try:
        yield from _parse_makefile_inc(makefile_inc)
    except Exception as err:
        raise RuntimeError(f"Failed to parse {makefile_inc}") from err


class YosysModule(NamedTuple):
    name: str
    sources: set[str]


def walk_yosys_modules(srcroot: Path) -> Iterator[YosysModule]:
    eprint(f"searching {srcroot} for modules...")
    yield YosysModule(name="kernel", sources=set(iter_kernel_sources()))
    for base_mod in YOSYS_BASE_MODULES:
        for dirpath, _, filenames in (srcroot / base_mod).walk():
            if YOSYS_MAKEFILE_INC in filenames:
                mod_name = dirpath.relative_to(srcroot).as_posix()
                mod_sources = set(parse_makefile_inc(dirpath / YOSYS_MAKEFILE_INC))
                yield YosysModule(mod_name, mod_sources)


def append_newline(line: str) -> str:
    return line + "\n"


def check_generate(command: str, *args: Any) -> None:
    proc = subprocess.run([command, *args])
    if (code := proc.returncode) != 0:
        stderr = proc.stderr.decode("utf-8")
        msg = f"`{command}` returned a non-zero exit code ({code}): {stderr}"
        raise RuntimeError(msg)


def generate_lexer(sourcefile: Path, outfile: Path) -> None:
    check_generate("flex", "-o", outfile, "-L", sourcefile)


def generate_parser(sourcefile: Path, outfile: Path) -> None:
    outname = outfile.with_suffix("")
    while len(outname.suffixes) > 0:
        outname = outname.with_suffix("")
    check_generate(
        "bison",
        "-o",
        outfile,
        "-l",
        "-d",
        "-b",
        outname,
        sourcefile,
    )


def generate_help(sourcefile: Path, outfile: Path) -> bool:
    helpbytes = subprocess.check_output([sys.executable, YOSYS_CELLHELP_PY, sourcefile])
    outdated = not outfile.exists() or outfile.read_bytes() != helpbytes
    if outdated:
        outfile.write_text(helpbytes.decode("utf-8"))
    return outdated


def main() -> int:
    yosys_lib_sources: dict[str, list[str]] = {}
    for module in walk_yosys_modules(YOSYS_ROOT_DIR):
        blacklisted = module.name in YOSYS_BLACKLISTED_MODULES
        if blacklisted or len(module.sources) == 0:
            continue
        libname = "Yosys" + "".join(
            part.capitalize() for part in module.name.split("/")
        )
        yosys_lib_sources[libname] = []
        for mod_source in module.sources:
            if mod_source in YOSYS_BLACKLISTED_SOURCES:
                continue
            yosys_lib_sources[libname].append(mod_source)
        yosys_lib_sources[libname].sort()
    yosys_lib_sources_serialized = json.dumps(
        yosys_lib_sources,
        indent=2,
        sort_keys=True,
    )
    yosys_lib_names = "".join(map(append_newline, sorted(yosys_lib_sources.keys())))
    yosys_lib_sources_json = VTS_YOSYS_SYS_DIR / "yosys_lib_sources.json"
    yosys_lib_names_txt = VTS_YOSYS_SYS_DIR / "yosys_lib_names.txt"
    lib_sources_should_update = (
        not yosys_lib_sources_json.exists()
        or yosys_lib_sources_json.read_text() != yosys_lib_sources_serialized
    )
    if lib_sources_should_update:
        eprint(f"updating {yosys_lib_sources_json}")
        yosys_lib_sources_json.write_text(yosys_lib_sources_serialized)
    lib_names_should_update = (
        not yosys_lib_names_txt.exists()
        or yosys_lib_names_txt.read_text() != yosys_lib_names
    )
    if lib_names_should_update:
        eprint(f"updating {yosys_lib_names_txt}")
        yosys_lib_names_txt.write_text(yosys_lib_names)
    if lib_sources_should_update or lib_names_should_update:
        (VTS_YOSYS_SYS_DIR / "CMakeLists.txt").touch()
    updated = lib_sources_should_update or lib_names_should_update
    for cellhelp in YOSYS_GENERATE_CELLHELP:
        help_source = YOSYS_TECHLIBS_COMMON / cellhelp
        help_outname = help_source.stem + "_help.inc"
        help_dest = YOSYS_BUILD_INCLUDE_DIR / "techlibs" / "common" / help_outname
        updated |= generate_help(help_source, help_dest)
    if not updated:
        eprint("nothing updated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
