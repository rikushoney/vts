from .vts_api_rs import PyComponent, PyPort


class Port:
    def __init__(self) -> None:
        self.port = PyPort()


class Component:
    def __init__(self) -> None:
        self.component = PyComponent()

    def add_port(self, name: str, port: Port) -> None:
        self.component.add_port(name, port.port)
