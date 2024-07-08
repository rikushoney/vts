#!/usr/bin/env python

import sys
from pathlib import Path
from typing import Any, Iterator, NamedTuple

VTS_ABC_SYS_DIR: Path = Path(__file__).parent
ABC_ROOT_DIR: Path = VTS_ABC_SYS_DIR / "abc"
ABC_SRC_DIR: Path = ABC_ROOT_DIR / "src"
ABC_MODULE_MAKE_FILENAME: str = "module.make"
ABC_BLACKLISTED_MODULES: set[str] = {
    "map/fpga",
    "misc/espresso",
    "opt/fsim",
    "phys/place",
    "proof/int2",
    "sat/bsat2",
}
ABC_BLACKLISTED_SOURCES: set[str] = {"base/main/main.c"}


def eprint(*args: Any, **kwargs: Any) -> None:
    kwargs["file"] = sys.stderr
    print(*args, **kwargs)


def process_line(line: str) -> Iterator[str]:
    def isvalid(entry: str) -> bool:
        return len(entry) > 0

    def cleanup(entry: str) -> str:
        return entry.strip("\\").strip()

    yield from filter(isvalid, map(cleanup, line.split()))


def parse_module_make(module_make_file: Path) -> Iterator[str]:
    try:
        lines = module_make_file.read_text().splitlines()
        i = 0
        while i < len(lines):
            line = lines[i].strip()
            if line.startswith("SRC"):
                needle = "+="
                jump = line.find(needle) + len(needle)
                if jump < len(needle):
                    raise ValueError(f'expected "{needle}" on line {i + 1}:"{line}"')
                line = line[jump:].lstrip()
                yield from process_line(line)
                while line.endswith("\\"):
                    i += 1
                    line = lines[i].strip()
                    yield from process_line(line)
            i += 1
    except Exception as err:
        raise RuntimeError(f"Failed to parse {module_make_file}") from err


class AbcModule(NamedTuple):
    name: str
    sources: set[str]


def walk_abc_modules(srcroot: Path) -> Iterator[AbcModule]:
    eprint(f"searching {srcroot} for modules...")
    for dirpath, _, filenames in srcroot.walk():
        if ABC_MODULE_MAKE_FILENAME in filenames:
            mod_name = dirpath.relative_to(srcroot).as_posix()
            if mod_name in ABC_BLACKLISTED_MODULES:
                continue
            mod_sources = set(parse_module_make(dirpath / ABC_MODULE_MAKE_FILENAME))
            yield AbcModule(mod_name, mod_sources)


def main() -> int:
    module_names: set[str] = set()
    c_sources: set[Path] = set()
    cxx_sources: set[Path] = set()
    for module in walk_abc_modules(ABC_SRC_DIR):
        if len(module.sources) == 0:
            continue
        module_names.add(module.name)
        for mod_source in module.sources:
            src = Path(mod_source)
            if src.as_posix() in ABC_BLACKLISTED_SOURCES:
                continue
            match src.suffix:
                case ".c":
                    c_sources.add(src)
                case ".cpp":
                    cxx_sources.add(src)
    module_names_list = "".join(f"{mod_name}\n" for mod_name in sorted(module_names))
    abc_c_sources_list = ";".join(f"{src.as_posix()}" for src in sorted(c_sources))
    abc_cxx_sources_list = ";".join(f"{src.as_posix()}" for src in sorted(cxx_sources))
    module_names_txt = VTS_ABC_SYS_DIR / "module_names.txt"
    abc_c_sources_txt = VTS_ABC_SYS_DIR / "abc_c_sources.txt"
    abc_cxx_sources_txt = VTS_ABC_SYS_DIR / "abc_cxx_sources.txt"
    sources_updated = False
    if (
        not module_names_txt.exists()
        or module_names_txt.read_text() != module_names_list
    ):
        module_names_txt.write_text(module_names_list)
        eprint(f"{module_names_txt} updated")
    if (
        not abc_c_sources_txt.exists()
        or abc_c_sources_txt.read_text() != abc_c_sources_list
    ):
        abc_c_sources_txt.write_text(abc_c_sources_list)
        eprint(f"{abc_c_sources_txt} updated")
        sources_updated = True
    if (
        not abc_cxx_sources_txt.exists()
        or abc_cxx_sources_txt.read_text() != abc_cxx_sources_list
    ):
        abc_cxx_sources_txt.write_text(abc_cxx_sources_list)
        eprint(f"{abc_cxx_sources_txt} updated")
        sources_updated = True
    if sources_updated:
        (VTS_ABC_SYS_DIR / "CMakeLists.txt").touch()
    else:
        eprint("nothing updated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
