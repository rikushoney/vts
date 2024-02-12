from enum import Enum


class Block:
    def __init__(self, name: str) -> None:
        # TODO: this should create a rust block instance
        self.name = name
        self.ports: dict[str, Port] = {}
        self.connections: list[ConnectionSpec] = []

    def add_port(
        self,
        name: str | object | None = None,
        port: Port | None = None,
        direction: PortDirection | None = None,
        n_pins: int | None = None
    ) -> None:
        if port:
            if not isinstance(port, Port):
                raise TypeError(f'port must be a Port instance, got "{type(port)}"')
            p = port.copy()
            if name is not None:
                p.name = name
            if direction is not None:
                p.direction = direction
            if n_pins is not None:
                p.n_pins = n_pins
        elif isinstance(name, Port):
            p = name.copy()
        elif name is None or direction is None:
            raise ValueError("port must have a name and direction")
        else:
            p = Port(name, direction, n_pins or 1)

        # TODO: this should call to rust code
        self.ports[p.name] = p

    def add_connection(
        self,
        input: Port | tuple[Port, PinRange] | Iterable[Port | tuple[Port, PinRange]],
        output: Port | tuple[Port, PinRange] | Iterable[Port | tuple[Port, PinRange]]
    ) -> None:
        # TODO: this should call to rust code
        self.connections.append(ConnectionSpec(input, output))


# TODO: this should be defined in rust code
class PortDirection(Enum):
    Input = 1
    Output = 2


class Port:
    def __init__(self, name: str, direction: PortDirection, n_pins: int = 1) -> None:
        self.name = name
        self.direction = direction
        self.n_pins = n_pins

    def __getitem__(self, range: int | slice) -> tuple[Port, PinRange]:
        if isinstance(range, int):
            return self, PinRange(range)
        elif isinstance(range, slice):
            if slice.step is not None:
                raise ValueError("port slicing with step is not supported")
            return self, PinRange((slice.start, slice.stop))
        else:
            raise TypeError("invalid port range")


# TODO: this should be defined in rust code
class PinRange:
    def __init__(
        self,
        index: int | None = None,
        range: tuple[int, int] | None = None
    ) -> None:
        self.storage: int | tuple[int, int] | None = None
        if index:
            self.storage = index
        elif range:
            self.storage = range


class ConnectionSpec:
    def __init__(
        self,
        input: Port | tuple[Port, PinRange] | Iterable[Port | tuple[Port, PinRange]],
        output: Port | tuple[Port, PinRange] | Iterable[Port | tuple[Port, PinRange]]
    ) -> None:
        self.input: list[tuple[Port, PinRange]] = []
        self.output: list[tuple[Port, PinRange]] = []
        if isinstance(input, Port):
            self.input.append((input, PinRange()))
        elif isinstance(input, tuple):
            self.input.append(input)
        else:
            for port in input:
                if isinstance(port, Port):
                    self.input.append((port, PinRange()))
                elif isinstance(port, tuple):
                    self.input.append(port)

        if isinstance(output, Port):
            self.output.append((output, PinRange()))
        elif isinstance(output, tuple):
            self.output.append(output)
        else:
            for port in output:
                if isinstance(port, Port):
                    self.output.append((port, PinRange()))
                elif isinstance(port, tuple):
                    self.output.append(port)


if __name__ == "__main__":
    b = Block("test")
    b.add_port("test_A", PortDirection.Input, n_pins=4)
    b.add_port("test_B", PortDirection.Output)
