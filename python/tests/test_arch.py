from vts.arch import Component, ComponentClass, Module, Port, PortClass, PortKind


def test_module():
    m = Module("test_mod")
    assert m.name == "test_mod"


def test_add_component():
    m = Module("_")
    c = m.add_component("test_comp", class_="LUT")
    assert c.name == "test_comp"
    assert c.class_ == ComponentClass.LUT
    assert c.name in m.components_dict().keys()

    tmp = Component("test_comp2", class_="LATCH")
    c2 = m.add_component(tmp)
    assert c2.name == "test_comp2"
    assert c2.class_ == ComponentClass.LATCH
    assert c2.name in m.components_dict().keys()
    assert c2 is not tmp

    c3 = m.add_component("test_comp3", component=c2)
    assert c2.name == "test_comp2"
    assert c3.name == "test_comp3"


def test_component():
    c = Component("test_comp", class_="LUT")
    assert c.name == "test_comp"

    c = Component("_", class_="LUT")
    assert c.class_ == ComponentClass.LUT


def test_add_port():
    c = Component("_")
    p = c.add_port("test_port", kind="i", class_="LUT_IN")
    assert p.name == "test_port"
    assert p.kind == PortKind.INPUT
    assert p.class_ == PortClass.LUT_IN
    assert p.name in c.ports_dict().keys()

    tmp = Port("test_port2", kind="o", class_="LATCH_OUT")
    p2 = c.add_port(tmp)
    assert p2.name == "test_port2"
    assert p2.kind == PortKind.OUTPUT
    assert p2.class_ == PortClass.LATCH_OUT
    assert p2.name in c.ports_dict().keys()
    assert p2 is not tmp

    p3 = c.add_port("test_port3", port=p2)
    assert p2.name == "test_port2"
    assert p3.name == "test_port3"
    assert p3.kind == PortKind.OUTPUT
    assert p3.class_ == PortClass.LATCH_OUT
    assert p3.name in c.ports_dict().keys()


def test_port():
    p = Port("test_port", "i")
    assert p.name == "test_port"
    assert p.kind == PortKind.INPUT

    p = Port("_", kind="i", n_pins=2)
    assert p.n_pins == 2

    p = Port("_", kind="i", class_="LUT_IN")
    assert p.class_ == PortClass.LUT_IN
