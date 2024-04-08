import sys

import vts._vts

sys.modules["vts._vts.arch"] = vts._vts.arch

from vts._vts.arch import (  # noqa
    PyComponent as Component,
    PyComponentClass as ComponentClass,
    PyModule as Module,
    PyPort as Port,
    PyPortClass as PortClass,
    PyPortKind as PortKind,
)

__all__ = ["Component", "ComponentClass", "Module", "Port", "PortClass", "PortKind"]
