#include <cstdio>
#include <string>
#include <vector>

#include "wrapper.h"

namespace Yosys {

namespace RTLIL {
struct Design;
}

void yosys_setup();

void yosys_shutdown();

extern RTLIL::Design *yosys_design;
extern std::vector<FILE *> log_files;
extern bool log_error_stderr;

RTLIL::Design *yosys_get_design();

void run_pass(std::string command, RTLIL::Design *design = nullptr);

bool run_frontend(std::string filename, std::string command,
                  RTLIL::Design *design = nullptr,
                  std::string *from_to_label = nullptr);

void run_backend(std::string filename, std::string command,
                 RTLIL::Design *design = nullptr);

} // namespace Yosys

void vts_yosys_setup() {
  Yosys::log_files.push_back(stdout);
  Yosys::log_error_stderr = true;
  Yosys::yosys_setup();
}

void vts_yosys_shutdown() { Yosys::yosys_shutdown(); }

YosysDesign vts_yosys_get_design() {
  return reinterpret_cast<YosysDesign>(Yosys::yosys_get_design());
}

void vts_yosys_run_pass(const char *command, YosysDesign design) {
  const auto command_str = std::string{command};
  const auto design_ptr = reinterpret_cast<Yosys::RTLIL::Design *>(design);
  Yosys::run_pass(command_str, design_ptr);
}

int vts_yosys_run_frontend(const char *filename, const char *command,
                           YosysDesign design) {
  const auto filename_str = std::string{filename};
  const auto command_str = std::string{command};
  const auto design_ptr = reinterpret_cast<Yosys::RTLIL::Design *>(design);
  return Yosys::run_frontend(filename_str, command_str, design_ptr) ? 0 : 1;
}

void vts_yosys_run_backend(const char *filename, const char *command,
                           YosysDesign design) {
  const auto filename_str = std::string{filename};
  const auto command_str = std::string{command};
  const auto design_ptr = reinterpret_cast<Yosys::RTLIL::Design *>(design);
  Yosys::run_backend(filename_str, command_str, design_ptr);
}
