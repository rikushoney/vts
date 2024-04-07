from __future__ import annotations

from enum import Enum
from typing import Mapping

class PyModule:
    def __new__(cls, name: str) -> PyModule: ...
    def copy(self) -> PyModule: ...
    def add_component(
        self,
        name_or_component: str | PyComponent | None = None,
        *,
        component: PyComponent | None = None,
        class_: PyComponentClass | None = None,
    ) -> PyComponent: ...

class PyComponentClass(Enum):
    LUT = ...
    LATCH = ...

class PyComponent:
    def add_port(self, name: str, port: PyPort) -> PyPort: ...
    def add_ports(self, ports: Mapping[str, PyPort]) -> None: ...
    def add_reference(
        self,
        component: PyComponent,
        alias: str | None = None,
        n_instances: int | None = None,
    ) -> PyComponentRef: ...
    def add_connection(
        self,
        source_pins: PyPortPins,
        sink_pins: PyPortPins,
        source_component: PyComponentRef | None = None,
        sink_component: PyComponentRef | None = None,
    ) -> PyConnection: ...

class PyComponentRef:
    def __init__(self, component: PyComponent, alias: str | None = None) -> None: ...
    @property
    def component(self) -> PyComponent: ...
    @component.setter
    def component(self, component: PyComponent) -> None: ...
    @property
    def alias(self) -> str | None: ...
    @alias.setter
    def alias(self, alias: str) -> None: ...
    @property
    def n_instances(self) -> int: ...
    @n_instances.setter
    def n_instances(self, n_instances: int) -> None: ...

class PyConnection:
    def __init__(
        self,
        source_pins: PyPortPins,
        sink_pins: PyPortPins,
        source_component: PyComponentRef | None = None,
        sink_component: PyComponentRef | None = None,
    ) -> None: ...
    @property
    def source_pins(self) -> PyPortPins: ...
    @source_pins.setter
    def source_pins(self, source_pins: PyPortPins) -> None: ...
    @property
    def source_component(self) -> PyComponentRef | None: ...
    @source_component.setter
    def source_component(self, source_component: PyComponentRef) -> None: ...
    @property
    def sink_pins(self) -> PyPortPins: ...
    @sink_pins.setter
    def sink_pins(self, sink_pins: PyPortPins) -> None: ...
    @property
    def sink_component(self) -> PyComponentRef | None: ...
    @sink_component.setter
    def sink_component(self, sink_component: PyComponentRef) -> None: ...

class PyPortKind(Enum):
    INPUT = ...
    OUTPUT = ...

class PyPortClass(Enum):
    CLOCK = ...
    LUT_IN = ...
    LUT_OUT = ...
    LATCH_IN = ...
    LATCH_OUT = ...

class PyPort:
    def __init__(
        self,
        name: str,
        kind: PyPortKind,
        n_pins: int | None = None,
        class_: PyPortClass | None = None,
    ) -> None: ...
    @property
    def name(self) -> str: ...
    @name.setter
    def name(self, value: str) -> None: ...
    @property
    def kind(self) -> PyPortKind: ...
    @kind.setter
    def kind(self, value: PyPortKind) -> None: ...
    @property
    def n_pins(self) -> int: ...
    @n_pins.setter
    def n_pins(self, value: int) -> None: ...
    @property
    def class_(self) -> PyPortClass: ...
    @class_.setter
    def class_(self, value: PyPortClass): ...
    def copy(self) -> PyPort: ...

class PyPortPins:
    def __init__(
        self,
        port: PyPort,
        start: int | None = None,
        end: int | None = None,
    ) -> None: ...
    @property
    def port(self) -> PyPort: ...
    @port.setter
    def port(self, port: PyPort) -> None: ...
    @property
    def start(self) -> int: ...
    @start.setter
    def start(self, start: int) -> None: ...
    @property
    def end(self) -> int: ...
    @end.setter
    def end(self, end: int) -> None: ...

def json_loads(input: str) -> PyModule: ...
def json_dumps(module: PyModule, pretty: bool) -> str: ...
