import sys

import vts._vts

sys.modules["vts._vts.arch"] = vts._vts.arch

from vts._vts.arch import (  # noqa
    PyComponent as Component,
    PyComponentClass as ComponentClass,
    PyComponentRef as ComponentRef,
    PyModule as Module,
    PyPort as Port,
    PyPortClass as PortClass,
    PyPortKind as PortKind,
    smoke_test,
)

__all__ = [
    "Component",
    "ComponentClass",
    "ComponentRef",
    "Module",
    "Port",
    "PortClass",
    "PortKind",
]
