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
    abc_sources: set[Path] = set()
    for module in walk_abc_modules(ABC_SRC_DIR):
        if len(module.sources) == 0:
            continue
        for mod_source in module.sources:
            src = Path(mod_source)
            if src.as_posix() in ABC_BLACKLISTED_SOURCES:
                continue
            match src.suffix:
                case ".c" | ".cpp":
                    abc_sources.add(src)
    abc_sources_list = ";".join(f"{src.as_posix()}" for src in sorted(abc_sources))
    abc_sources_txt = VTS_ABC_SYS_DIR / "abc_sources.txt"
    sources_updated = False
    if not abc_sources_txt.exists() or abc_sources_txt.read_text() != abc_sources_list:
        abc_sources_txt.write_text(abc_sources_list)
        eprint(f"{abc_sources_txt} updated")
        sources_updated = True
    if sources_updated:
        (VTS_ABC_SYS_DIR / "CMakeLists.txt").touch()
    else:
        eprint("nothing updated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
