from math import floor, log10

from vts import arch
from vts.arch import concat

m = arch.Module("m")

c1 = m.add_component("c1")
c1.add_port("p1", kind="i")

c2 = m.add_component("c2")
c2.add_port("p2", kind="i")
c1.add_reference(c2)

c1.c2.p2 = c1.p1

c3 = m.add_component("c3")
c3.add_port("p3", kind="i", n_pins=2)
c1.add_reference(c3)

c1.c3.p3[0] = c1.c2.p2

c4 = m.add_component("c4")
c4.add_port("p4", kind="o")
c1.add_reference(c4)

c1.c4.p4 = c1.c3.p3[1]

c1.add_port("o", kind="o", n_pins=2)

c1.o = concat(c1.c4.p4, c1.c3.p3[0])

c1.c3.p3[1] = c1.o


def print_with_linum(s: str) -> None:
    n_lines = s.count("\n") + 1
    margin = floor(log10(n_lines)) + 1

    for i, line in enumerate(s.splitlines()):
        linum = i + 1
        print(f"{linum: >{margin}}. {line}")


dump1 = arch.json_dumps(m, True)

print("Json:")
print_with_linum(dump1)

dump2 = arch.yaml_dumps(arch.json_loads(dump1))

print("Yaml:")
print_with_linum(dump2)

dump3 = arch.toml_dumps(arch.yaml_loads(dump2), pretty=True)

print("Toml:")
print_with_linum(dump3)

_ = arch.toml_loads(dump3)
