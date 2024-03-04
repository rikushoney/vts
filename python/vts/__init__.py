from __future__ import annotations

from typing import Iterable, Literal

from vts._vts_api_rs import (
    PyComponent as _Component,
    PyComponentClass as ComponentClass,
    PyModule_ as _Module,
    PyPort as _Port,
    PyPortClass as PortClass,
    PyPortKind as PortKind,
)


def _component_class_from_str(class_str: str) -> ComponentClass:
    class_ = class_str.lower()

    if class_ == "lut":
        return ComponentClass.LUT
    elif class_ in ("latch", "ff"):
        return ComponentClass.LATCH

    raise ValueError(f'unknown component class "{class_str}"')


_ComponentClassStr = Literal["lut", "LUT", "latch", "LATCH", "ff", "FF"]


def _port_kind_from_str(kind_str: str) -> PortKind:
    kind = kind_str.lower()

    if kind in ["input", "in", "i"]:
        return PortKind.INPUT
    elif kind in ["output", "out", "o"]:
        return PortKind.OUTPUT

    raise ValueError(f'unknown port kind "{kind_str}"')


_PortKindStr = Literal[
    "input", "in", "i", "INPUT", "IN", "I", "output", "out", "o", "OUTPUT", "OUT", "O"
]


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

    raise ValueError(f'unknown port class "{class_str}"')


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


class Module:
    def __init__(self, name: str) -> None:
        self._module = _Module(name)

    @property
    def name(self) -> str:
        return self._module.name

    def components_dict(self) -> dict[str, Component]:
        return {
            component.name: Component._ref(component)
            for component in self._module.components.values()
        }

    def components_list(self) -> list[Component]:
        return [
            Component._ref(component) for component in self._module.components.values()
        ]

    def add_component(
        self,
        name: str | Component | None = None,
        *,
        component: Component | None = None,
        class_: ComponentClass | _ComponentClassStr | None = None,
    ) -> Component:
        if component is not None:
            component = component.copy()

            if name is not None:
                if not isinstance(name, str):
                    raise TypeError(f'expected "name" to be "str" not "{type(name)}"')
                component._component.name = name
            if class_ is not None:
                if isinstance(class_, str):
                    class_ = _component_class_from_str(class_)
                component._component.class_ = class_
        elif isinstance(name, Component):
            component = name.copy()
        else:
            if name is None:
                raise ValueError("component must have a name")

            component = Component(name)

        self._module.add_component(component._component.name, component._component)

        return component

    def add_components(
        self, components: Iterable[Component] | dict[str, Component]
    ) -> None:
        if not isinstance(components, dict):
            for component in components:
                self.add_component(component)
        else:
            for name, component in components.items():
                self.add_component(name, component=component)


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
        elif isinstance(name, Port):
            port = name.copy()
        else:
            if name is None:
                raise ValueError("port must have a name")
            if kind is None:
                raise ValueError("port must have a kind")

            port = Port(name, kind, n_pins, class_)

        self._component.add_port(port._port.name, port._port)

        return port

    def add_ports(self, ports: Iterable[Port] | dict[str, Port]) -> None:
        if not isinstance(ports, dict):
            for port in ports:
                self.add_port(port)
        else:
            for name, port in ports.items():
                self.add_port(name, port=port)

    @classmethod
    def _ref(cls, component: _Component) -> Component:
        c = cls.__new__(cls)
        c._component = component
        return c

    def copy(self, name: str | None = None) -> Component:
        component = Component(name or self.name, self.class_)

        component.add_ports(self.ports_dict())

        return component


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
