VERILOG_FLAGS ?= -g2012

_build:
	mkdir $@

testbench: testbench.sv _build crc8.sv crc16.sv
	iverilog $(VERILOG_FLAGS) -o _build/$@ $(<F)

simulate: testbench
	vvp _build/$(<F)

synth:
	yosys synth.ys
	head -n -2 crc8.aag > crc8_trim.aag
	head -n -2 crc16.aag > crc16_trim.aag

clean:
	rm -r _build crc_testbench_dump.vcd

.PHONY: clean
