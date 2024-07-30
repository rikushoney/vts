typedef void *YosysDesign;

extern "C" {
void vts_yosys_setup();

void vts_yosys_shutdown();

YosysDesign vts_yosys_get_design();

void vts_yosys_run_pass(const char *command, YosysDesign design);

int vts_yosys_run_frontend(const char *filename, const char *command,
                           YosysDesign design);

void vts_yosys_run_backend(const char *filename, const char *command,
                           YosysDesign design);
}
