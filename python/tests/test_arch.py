from vts import Component


def test_smoke():
    _c = Component("test")
    a = 1
    b = 2
    assert a + b == 3
