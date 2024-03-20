from __future__ import annotations

from collections.abc import Mapping
from typing import Iterable

from vts._vts_api_rs import (
    PyModule as _Module,
    json_dumps as _json_dumps,
    json_loads as _json_loads,
)
from vts.arch.component import (
    Component,
    ComponentClass,
    _component_class_from_str,
    _ComponentClassStr,
)


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

    def __str__(self) -> str:
        return f'Module(name="{self.name}, components={{...}}")'


def json_dumps(module: Module, pretty: bool = False) -> str:
    return _json_dumps(module._module, pretty)


def json_loads(input: str) -> Module:
    module = _json_loads(input)
    return Module._wrap(module)
