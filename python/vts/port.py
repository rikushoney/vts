from enum import Enum


class PortDirection(Enum):
    INPUT = 0
    OUTPUT = 1


class Port:
    def __init__(self, name: str, direction: PortDirection) -> None:
        self._name = name
        self._direction = direction
