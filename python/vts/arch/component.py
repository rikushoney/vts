from __future__ import annotations

from collections.abc import Mapping
from typing import Iterable, Literal

from vts._vts_api_rs import (
    PyComponent as _Component,
    PyComponentClass as ComponentClass,
    PyComponentRef as _ComponentRef,
    PyConnection as _Connection,
    PyPortClass as PortClass,
    PyPortKind as PortKind,
)
from vts.arch.port import (
    PinRange,
    Port,
    _port_class_from_str,
    _port_kind_from_str,
    _PortClassStr,
    _PortKindStr,
)


def _component_class_from_str(class_: str) -> ComponentClass:
    match class_.lower():
        case "lut":
            return ComponentClass.LUT
        case "latch" | "ff":
            return ComponentClass.LATCH
        case _:
            raise ValueError(f'unknown component class "{class_}"')


_ComponentClassStr = Literal["lut", "LUT", "latch", "LATCH", "ff", "FF"]


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
        return {port.name: Port._wrap(port) for port in self._component.ports.values()}

    def ports_list(self) -> list[Port]:
        return [Port._wrap(port) for port in self._component.ports.values()]

    def add_port(
        self,
        name: str | Port | None = None,
        *,
        port: Port | None = None,
        kind: PortKind | _PortKindStr | None = None,
        n_pins: int | None = None,
        class_: PortClass | _PortClassStr | None = None,
    ) -> Port:
        if port is not None:
            port = port.copy()

            if name is not None:
                if not isinstance(name, str):
                    raise TypeError(f'expected "name" to be "str" not "{type(name)}"')
                port._port.name = name

            if kind is not None:
                if isinstance(kind, str):
                    kind = _port_kind_from_str(kind)
                port._port.kind = kind

            if n_pins is not None:
                port._port.n_pins = n_pins

            if class_ is not None:
                if isinstance(class_, str):
                    class_ = _port_class_from_str(class_)
                port._port.class_ = class_
        else:
            match name:
                case Port():
                    port = name.copy()
                case name if isinstance(name, str):
                    if kind is None:
                        raise ValueError("port must have a kind")
                    port = Port(name, kind, n_pins, class_)
                case _:
                    raise ValueError("port must have a name")

        return Port._wrap(self._component.add_port(port._port.name, port=port._port))

    def add_ports(self, ports: Iterable[Port] | Mapping[str, Port]) -> None:
        if isinstance(ports, Mapping):
            for name, port in ports.items():
                self.add_port(name, port=port)
        else:
            for port in ports:
                self.add_port(port)

    def add_reference(
        self,
        component: Component,
        *,
        alias: str | None = None,
        n_instances: int | None = None,
    ) -> ComponentRef:
        return ComponentRef._wrap(
            self._component.add_reference(component._component, alias, n_instances)
        )

    def add_connection(self, source: PinRange, sink: PinRange) -> Connection:
        return Connection._wrap(
            self._component.add_connection(source._range, sink._range)
        )

    @classmethod
    def _wrap(cls, component: _Component) -> Component:
        c = cls.__new__(cls)
        c._component = component
        return c

    def copy(self, name: str | None = None) -> Component:
        component = Component._wrap(self._component.copy())

        if name is not None:
            component._component.name = name

        return component

    def __str__(self) -> str:
        class_ = str(self.class_) if self.class_ is not None else "None"
        return f'Component(name="{self.name}", ports={{...}}, class={class_})'


class ComponentRef:
    def __init__(self, component: Component, alias: str | None = None) -> None:
        self._reference = _ComponentRef(component._component, alias)

    @classmethod
    def _wrap(cls, reference: _ComponentRef) -> ComponentRef:
        ref = cls.__new__(cls)
        ref._reference = reference
        return ref


class Connection:
    def __init__(self, source: PinRange, sink: PinRange) -> None:
        self._connection = _Connection(source._range, sink._range)

    @classmethod
    def _wrap(cls, connection: _Connection) -> Connection:
        c = cls.__new__(cls)
        c._connection = connection
        return c
