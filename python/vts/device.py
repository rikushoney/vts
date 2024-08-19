from .cell import Cell
from .property import Property


class Device:
    def __init__(self, name: str) -> None:
        self._name = name
        self._cells: dict[str, Cell] = {}
        self._properties: dict[str, Property] = {}
