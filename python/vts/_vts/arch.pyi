from __future__ import annotations

from typing import Any, Literal, TypeAlias, overload

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
    "FF_IN",
    "ff_out",
    "FF_OUT",
]

_ConnectionKindStr = Literal["direct", "DIRECT", "complete", "COMPLETE", "mux", "MUX"]

class ComponentClass:
    LUT = ...
    LATCH = ...

class PortKind:
    INPUT = ...
    OUTPUT = ...

class PortClass:
    LUT_IN = ...
    LUT_OUT = ...
    LATCH_IN = ...
    LATCH_OUT = ...

class ConnectionKind:
    DIRECT = ...
    COMPLETE = ...
    MUX = ...

_ComponentClass: TypeAlias = _ComponentClassStr | ComponentClass

_PortKind: TypeAlias = _PortKindStr | PortKind

_PortClass: TypeAlias = _PortClassStr | PortClass

_ConnectionKind: TypeAlias = _ConnectionKindStr | ConnectionKind

class Module:
    def __new__(cls, name: str) -> Module: ...
    def name(self) -> str: ...
    def copy(self, name: str | None = None) -> Module: ...
    @overload
    def add_component(
        self,
        name: str,
        *,
        component: Component | None = None,
        class_: _ComponentClass | None = None,
    ) -> Component: ...
    @overload
    def add_component(
        self,
        name: Component,
        *,
        class_: _ComponentClass | None = None,
    ) -> Component: ...
    @overload
    def add_component(
        self,
        *,
        component: Component,
        class_: _ComponentClass | None = None,
    ) -> Component: ...

class Component:
    def module(self) -> Module: ...
    def name(self) -> str: ...
    def class_(self) -> ComponentClass: ...
    @overload
    def add_port(
        self,
        name: str,
        *,
        port: Port | None = None,
        kind: _PortKind | None = None,
        n_pins: int | None = None,
        class_: _PortClass | None = None,
    ) -> Port: ...
    @overload
    def add_port(
        self,
        name: Port,
        *,
        kind: _PortKind | None = None,
        n_pins: int | None = None,
        class_: _PortClass | None = None,
    ) -> Port: ...
    @overload
    def add_port(
        self,
        *,
        port: Port,
        kind: _PortKind | None = None,
        n_pins: int | None = None,
        class_: _PortClass | None = None,
    ) -> Port: ...
    def add_reference(
        self,
        component: Component,
        *,
        alias: str | None = None,
        n_instances: int | None = None,
    ) -> ComponentRef: ...
    def add_connection(
        self,
        source: Signature,
        sink: Signature,
        *,
        kind: _ConnectionKind | None = None,
    ) -> None: ...
    def __getattr__(self, port_or_reference: str) -> Any: ...
    def __setattr__(self, sink: str, source: Connector) -> None: ...

class Port:
    def module(self) -> Module: ...
    def name(self) -> str: ...
    def kind(self) -> PortKind: ...
    def n_pins(self) -> int: ...
    def class_(self) -> PortClass: ...
    def select(self, index: slice | int) -> PortPins: ...
    def __getitem__(self, index: slice | int) -> Signature: ...
    def __setitem__(self, sink: slice | int, source: Connector) -> None: ...

class ComponentRef:
    def module(self) -> Module: ...
    def component(self) -> Component: ...
    def alias(self) -> str | None: ...
    def alias_or_name(self) -> str: ...
    def n_instances(self) -> int: ...
    def select(self, index: slice | int) -> ComponentRefSelection: ...
    def __getitem__(self, index: slice | int) -> ComponentRefSelection: ...
    def __getattr__(self, port: str) -> ComponentRefPort: ...
    def __setattr__(self, sink: str, source: Connector) -> None: ...

class PortPins:
    pass

class ComponentRefPort:
    def __getitem__(self, index: slice | int) -> Signature: ...
    def __setitem__(self, sink: slice | int, source: Connector) -> None: ...

class Signature:
    pass

class ComponentRefSelection:
    def __getattr__(self, port: str) -> Signature: ...
    def __setattr__(self, sink: str, source: Connector) -> None: ...

IntoSignature: TypeAlias = Signature | ComponentRefPort | Port

class Direct:
    pass

def direct(connector: Connector) -> Direct: ...

class Complete:
    pass

def complete(connector: Connector) -> Complete: ...

class Mux:
    pass

def mux(connector: Connector) -> Mux: ...

class Concat:
    pass

Connector: TypeAlias = IntoSignature | Direct | Complete | Mux | Concat

def concat(*connectors) -> Concat: ...
def json_dumps(module: Module, pretty: bool) -> str: ...
def json_loads(input: str) -> Module: ...
def yaml_dumps(module: Module) -> str: ...
def yaml_loads(input: str) -> Module: ...
def toml_dumps(module: Module, pretty: bool) -> str: ...
def toml_loads(input: str) -> Module: ...
