from __future__ import annotations

from typing import Iterable, Literal

from vts_api._vts_api_rs import (
    PyComponent as _Component,
    PyComponentClass as ComponentClass,
    PyPort as _Port,
    PyPortClass as PortClass,
    PyPortKind as PortKind,
)


def _component_class_from_str(class_str: str) -> ComponentClass:
    class_ = class_str.lower()
    if class_ == "lut":
        return ComponentClass.LUT
    elif class_ == "latch":
        return ComponentClass.LATCH

    raise ValueError(f'invalid component class "{class_str}"')


_ComponentClassStr = Literal["lut", "LUT", "latch", "LATCH"]


class Component:
    def __init__(
        self, name: str, class_: ComponentClass | _ComponentClassStr | None = None
    ) -> None:
        if isinstance(class_, str):
            class_ = _component_class_from_str(class_)

        self._component = _Component(name, class_)

    @property
    def name(self) -> str:
        return self._component.name

    @property
    def class_(self) -> ComponentClass | None:
        return self._component.class_

    def ports_dict(self) -> dict[str, Port]:
        return {port.name: Port._ref(port) for port in self._component.ports.values()}

    def ports_list(self) -> list[Port]:
        return [Port._ref(port) for port in self._component.ports.values()]

    def add_port(
        self,
        name: str | Port | None = None,
        *,
        port: Port | None = None,
        kind: PortKind | None = None,
        n_pins: int | None = None,
        class_: PortClass | None = None,
    ) -> Port:
        if port is not None:
            port = port.copy()

            if name is not None:
                if not isinstance(name, str):
                    raise TypeError(f'expected "name" to be "str" not "{type(name)}"')
                port._port.name = name
            if kind is not None:
                port._port.kind = kind
            if n_pins is not None:
                port._port.n_pins = n_pins
            if class_ is not None:
                port._port.class_ = class_
        elif isinstance(name, Port):
            port = name.copy()
        else:
            if name is None:
                raise ValueError("port must have a name")
            if kind is None:
                raise ValueError("port must have a kind")

            port = Port(name, kind, n_pins, class_)

        if port.name in self._component.ports:
            raise ValueError(f'port with name "{port.name}" already in "{self.name}"')
        self._component.add_port(port._port.name, port._port)

        return port

    def add_ports(self, ports: Iterable[Port] | dict[str, Port]) -> None:
        if not isinstance(ports, dict):
            for port in ports:
                self.add_port(port)
        else:
            for name, port in ports.items():
                self.add_port(name, port=port)

    def copy(self, name: str | None = None) -> Component:
        component = Component(name or self.name, self.class_)
        component.add_ports(self.ports_dict())

        return component


def _port_kind_from_str(kind_str: str) -> PortKind:
    kind = kind_str.lower()
    if kind in ["input", "in"]:
        return PortKind.INPUT
    elif kind in ["output", "out"]:
        return PortKind.OUTPUT

    raise ValueError(f'invalid port kind "{kind_str}"')


def _port_class_from_str(class_str: str) -> PortClass:
    class_ = class_str.lower()
    if class_ == "lut_in":
        return PortClass.LUT_IN
    elif class_ == "lut_out":
        return PortClass.LUT_OUT
    elif class_ == "latch_in":
        return PortClass.LATCH_IN
    elif class_ == "latch_out":
        return PortClass.LATCH_OUT

    raise ValueError(f'invalid port class "{class_str}"')


_PortKindStr = Literal["input", "in", "INPUT", "IN", "output", "out", "OUTPUT", "OUT"]
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
