from vts import Component, ComponentClass, Module, Port, PortClass, PortKind


def test_module():
    m = Module("test_mod")
    assert m.name == "test_mod"


def test_add_component():
    m = Module("_")
    c = m.add_component("test_comp")
    assert c.name == "test_comp"
    assert c.name in m.components_dict().keys()


def test_component():
    c = Component("test_comp")
    assert c.name == "test_comp"

    c = Component("_", class_="LUT")
    assert c.class_ == ComponentClass.LUT


def test_add_port():
    c = Component("_")
    p = c.add_port("test_port", kind="i")
    assert p.name == "test_port"
    assert p.name in c.ports_dict().keys()


def test_port():
    p = Port("test_port", "i")
    assert p.name == "test_port"
    assert p.kind == PortKind.INPUT

    p = Port("_", kind="i", n_pins=2)
    assert p.n_pins == 2

    p = Port("_", kind="i", class_="LUT_IN")
    assert p.class_ == PortClass.LUT_IN
