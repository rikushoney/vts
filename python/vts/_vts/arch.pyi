from __future__ import annotations

from typing import Literal

_ComponentClassStr = Literal["lut", "LUT", "latch", "LATCH", "ff", "FF"]

_PortKindStr = Literal[
    "i", "I", "in", "IN", "input", "input", "o", "O", "out", "OUT", "output", "OUTPUT"
]

_PortClassStr = Literal[
    "lut_in",
    "LUT_IN",
    "lut_out",
    "LUT_OUT",
    "latch_in",
    "LATCH_IN",
    "latch_out",
    "LATCH_OUT",
    "ff_in",
    "FF_IN" "ff_out",
    "FF_OUT",
]

class PyModule:
    def __new__(cls, name: str) -> PyModule: ...
    def name(self) -> str: ...
    def copy(self, name: str | None = None) -> PyModule: ...
    def add_component(
        self,
        name: str | PyComponent | None = None,
        *,
        component: PyComponent | None = None,
        class_: PyComponentClass | _ComponentClassStr | None = None,
    ) -> PyComponent: ...

class PyComponentClass:
    LUT = ...
    LATCH = ...

class PyComponent:
    def module(self) -> PyModule: ...
    def name(self) -> str: ...
    def class_(self) -> PyComponentClass: ...
    def add_port(
        self,
        name: str | PyPort | None = None,
        *,
        port: PyPort | None = None,
        kind: PyPortKind | _PortKindStr | None = None,
        n_pins: int | None = None,
        class_: PyPortClass | _PortClassStr | None = None,
    ) -> PyPort: ...

class PyPortKind:
    INPUT = ...
    OUTPUT = ...

class PyPortClass:
    LUT_IN = ...
    LUT_OUT = ...
    LATCH_IN = ...
    LATCH_OUT = ...

class PyPort:
    def module(self) -> PyModule: ...
    def name(self) -> str: ...
    def kind(self) -> PyPortKind: ...
    def n_pins(self) -> int: ...
    def class_(self) -> PyPortClass: ...
