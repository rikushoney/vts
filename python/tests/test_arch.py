from math import floor, log10

from vts import arch


def print_with_linum(s: str) -> None:
    n_lines = s.count("\n") + 1
    margin = floor(log10(n_lines)) + 1

    for i, line in enumerate(s.splitlines()):
        linum = i + 1
        print(f"{linum: >{margin}}. {line}")


def test_arch():
    m = arch.Module("test_mod")

    c1 = m.add_component("test_comp_1")
    c1.add_port("test_port_1", kind="i")

    c2 = m.add_component("test_comp_2")
    c2.add_port("test_port_2", kind="i")
    c1_c2 = c1.add_reference(c2)

    c1.test_port_1 = c1_c2.test_port_2[:]

    c3 = m.add_component("test_comp_3")
    c3.add_port("test_port_3", kind="i", n_pins=2)
    c1_c3 = c1.add_reference(c3)

    c1_c2.test_port_2 = c1_c3.test_port_3[0]

    c4 = m.add_component("test_comp_4")
    c4.add_port("test_port_4", kind="o")
    c1_c4 = c1.add_reference(c4, alias="c4")

    c1_c3.test_port_3[1] = c1_c4.test_port_4

    dump1 = arch.json_dumps(m, True)

    print("Initial:")
    print_with_linum(dump1)

    dump2 = arch.json_dumps(arch.json_loads(dump1), pretty=True)

    print("Reloaded:")
    print_with_linum(dump2)
