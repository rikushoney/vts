from __future__ import annotations

from typing import Literal

from vts._vts_api_rs import (
    PyPinRange as _PinRange,
    PyPort as _Port,
    PyPortClass as PortClass,
    PyPortKind as PortKind,
)


def _port_kind_from_str(kind: str) -> PortKind:
    match kind.lower():
        case "input" | "in" | "i":
            return PortKind.INPUT
        case "output" | "out" | "o":
            return PortKind.OUTPUT
        case _:
            raise ValueError(f'unknown port kind "{kind}"')


_PortKindStr = Literal[
    "input", "in", "i", "INPUT", "IN", "I", "output", "out", "o", "OUTPUT", "OUT", "O"
]


def _port_class_from_str(class_: str) -> PortClass:
    match class_.lower():
        case "lut_in":
            return PortClass.LUT_IN
        case "lut_out":
            return PortClass.LUT_OUT
        case "latch_in" | "ff_in":
            return PortClass.LATCH_IN
        case "latch_out" | "ff_out":
            return PortClass.LATCH_OUT
        case _:
            raise ValueError(f'unknown port class "{class_}"')


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
    "FF_IN",
    "FF_OUT",
    "ff_out",
]


class Port:
    def __init__(
        self,
        name: str,
        kind: PortKind | _PortKindStr,
        n_pins: int | None = None,
        class_: PortClass | _PortClassStr | None = None,
    ) -> None:
        if isinstance(kind, str):
            kind = _port_kind_from_str(kind)

        if isinstance(class_, str):
            class_ = _port_class_from_str(class_)

        self._port = _Port(name, kind, n_pins, class_)

    @property
    def name(self) -> str:
        return self._port.name

    @property
    def kind(self) -> PortKind:
        return self._port.kind

    @property
    def n_pins(self) -> int:
        return self._port.n_pins

    @property
    def class_(self) -> PortClass | None:
        return self._port.class_

    @classmethod
    def _wrap(cls, port: _Port) -> Port:
        p = cls.__new__(cls)
        p._port = port
        return p

    def copy(self, name: str | None = None) -> Port:
        port = Port._wrap(self._port.copy())

        if name is not None:
            port._port.name = name

        return port

    def __repr__(self) -> str:
        return str(self)

    def __str__(self) -> str:
        class_ = str(self.class_) if self.class_ is not None else "None"
        return f'Port(name="{self.name}", kind={self.kind}, class={class_})'


class PinRange:
    def __init__(
        self, port: Port, start: int | None = None, end: int | None = None
    ) -> None:
        self._range = _PinRange(port._port, start, end)

    @classmethod
    def _wrap(cls, range: _PinRange) -> PinRange:
        r = cls.__new__(cls)
        r._range = range
        return r
