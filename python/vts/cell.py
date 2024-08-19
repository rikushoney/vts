from collections.abc import Iterator
from enum import Enum

from .placement import Placement
from .port import Port
from .property import Property


class Signature:
    def __init__(self, inputs: Iterator[Port], outputs: Iterator[Port]):
        self._inputs = list(inputs)
        self._outputs = list(outputs)


class CellKind(Enum):
    LOOKUP_TABLE = 0
    LATCH = 1


class Cell:
    def __init__(self, name: str, kind: CellKind, signature: Signature):
        self._name = name
        self._kind = kind
        self._signature = signature
        self._placements: dict[str, Placement] = {}
        self._properties: dict[str, Property] = {}
