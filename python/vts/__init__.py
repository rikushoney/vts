from __future__ import annotations

from typing import Literal

from vts._vts_api_rs import (
    PyComponent as Component,
    PyComponentClass as ComponentClass,
    PyModule_ as Module,
    PyPort as _Port,
    PyPortClass as PortClass,
    PyPortKind as PortKind,
    _port_class_from_str,
    _port_kind_from_str,
)

_ComponentClassStr = Literal["lut", "LUT", "latch", "LATCH", "ff", "FF"]


_PortKindStr = Literal[
    "input", "in", "i", "INPUT", "IN", "I", "output", "out", "o", "OUTPUT", "OUT", "O"
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
    def _ref(cls, port: _Port) -> Port:
        p = cls.__new__(cls)
        p._port = port
        return p

    def copy(self, name: str | None = None) -> Port:
        return Port(
            name or self._port.name,
            self._port.kind,
            self._port.n_pins,
            self._port.class_,
        )


__all__ = ["Module", "Component", "ComponentClass", "Port", "PortKind", "PortClass"]
