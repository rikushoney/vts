from vts_api._vts_api_rs import (
    PyComponent as _Component,
    PyComponentClass as ComponentClass,
    PyPort as _Port,
    PyPortKind as PortKind,
    PyPortClass as PortClass,
)


from typing import Iterable


class Port:
    def __init__(
        self,
        name: str,
        kind: PortKind,
        n_pins: int | None = None,
        class_: PortClass | None = None,
    ) -> None:
        self._port = _Port(name, kind, n_pins, class_)

    def copy(self) -> "Port":
        return Port(
            self._port.name, self._port.kind, self._port.n_pins, self._port.class_
        )


class Component:
    def __init__(self, name: str, class_: ComponentClass | None = None) -> None:
        self._component = _Component(name, class_)

    def add_port(
        self,
        name: str | Port | None = None,
        *,
        port: Port | None = None,
        kind: PortKind | None = None,
        n_pins: int | None = None,
        class_: PortClass | None = None,
    ) -> None:
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

        self._component.add_port(port._port.name, port._port)

    def add_ports(self, ports: Iterable[Port] | dict[str, Port]) -> None:
        if not isinstance(ports, dict):
            ports = {port._port.name: port.copy() for port in ports}

        self._component.add_ports(ports)
