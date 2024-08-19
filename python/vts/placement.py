from .cell import Cell
from .property import Property


class Placement:
    def __init__(self, cell: Cell) -> None:
        self._cell = cell
        self._properties: dict[str, Property] = {}
