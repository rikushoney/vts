typedef enum logic [0] {SHIFT_LEFT, SHIFT_RIGHT} SHIFT_OP;

module sr #(parameter WIDTH = 4) (
  input logic             clk,
  input logic             reset,
  input SHIFT_OP          op,
  input logic             shift_in,
  output logic [WIDTH-1:0] shift_out,
);

logic [WIDTH-1:0] registers;

always_ff @(posedge clk, negedge reset) begin
  if (~reset) begin
    registers <= 0;
  end else if (op == SHIFT_LEFT) begin
    registers <= {registers[WIDTH-2:0], shift_in};
  end else if (op == SHIFT_RIGHT) begin
    registers <= {shift_in, registers[WIDTH-1:1]};
  end
end

assign shift_out = registers;

endmodule: sr
