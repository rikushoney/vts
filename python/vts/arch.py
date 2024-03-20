from __future__ import annotations

from collections.abc import Mapping
from typing import Iterable, Literal

from vts._vts_api_rs import (
    PyComponent as _Component,
    PyComponentClass as ComponentClass,
    PyModule as _Module,
    PyPort as _Port,
    PyPortClass as PortClass,
    PyPortKind as PortKind,
    json_dumps as _json_dumps,
    json_loads as _json_loads,
)


def _component_class_from_str(class_: str) -> ComponentClass:
    match class_.lower():
        case "lut" | "ff":
            return ComponentClass.LUT
        case "latch":
            return ComponentClass.LATCH
        case _:
            raise ValueError(f'unknown component class "{class_}"')


_ComponentClassStr = Literal["lut", "LUT", "latch", "LATCH", "ff", "FF"]


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


class Module:
    def __init__(self, name: str) -> None:
        self._module = _Module(name)

    @property
    def name(self) -> str:
        return self._module.name

    def components_dict(self) -> dict[str, Component]:
        return {
            component.name: Component._wrap(component)
            for component in self._module.components.values()
        }

    def components_list(self) -> list[Component]:
        return [
            Component._wrap(component) for component in self._module.components.values()
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
        else:
            match name:
                case Component():
                    component = name.copy()
                case name if isinstance(name, str):
                    component = Component(name, class_)
                case _:
                    raise ValueError("component must have a name")

        return Component._wrap(
            self._module.add_component(component._component.name, component._component)
        )

    def add_components(
        self, components: Iterable[Component] | Mapping[str, Component]
    ) -> None:
        if isinstance(components, Mapping):
            for name, component in components.items():
                self.add_component(name, component=component)
        else:
            for component in components:
                self.add_component(component)

    @classmethod
    def _wrap(cls, module: _Module) -> Module:
        m = cls.__new__(cls)
        m._module = module
        return m

    def copy(self, name: str | None = None) -> Module:
        module = self._module.copy()

        if name is not None:
            module.name = name

        return Module._wrap(module)


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


def json_dumps(module: Module, pretty: bool = False) -> str:
    return _json_dumps(module._module, pretty)


def json_loads(input: str) -> Module:
    module = _json_loads(input)
    return Module._wrap(module)
