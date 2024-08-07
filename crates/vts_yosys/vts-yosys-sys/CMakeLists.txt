cmake_minimum_required(VERSION 3.20)

string(TIMESTAMP vts_yosys_calver "%Y%m%d" UTC)

project(
  vts_yosys
  VERSION 0.0.${vts_yosys_calver}
  LANGUAGES C CXX
)

option(VTS_YOSYS_BUILD_DYLIB "Build VTS Yosys shared library" OFF)

set(vts_yosys_required_cxx_standard 17)

set(gcc_like_cxx "$<COMPILE_LANG_AND_ID:CXX,AppleClang,Clang,GNU>")
set(gcc_like_linker "$<LINK_LANG_AND_ID:CXX,AppleClang,Clang,GNU>")

set(
  gcc_like_compile_options
  "-Wall"
  "-Wextra"
)

set(CMAKE_CXX_STANDARD ${vts_yosys_required_cxx_standard})
set(CMAKE_CXX_STANDARD_REQUIRED YES)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(GNUInstallDirs)

set(
  CMAKE_ARCHIVE_OUTPUT_DIRECTORY
  "${CMAKE_BINARY_DIR}/${CMAKE_INSTALL_LIBDIR}"
)

set(
  CMAKE_LIBRARY_OUTPUT_DIRECTORY
  "${CMAKE_BINARY_DIR}/${CMAKE_INSTALL_LIBDIR}"
)

set(
  CMAKE_RUNTIME_OUTPUT_DIRECTORY
  "${CMAKE_BINARY_DIR}/${CMAKE_INSTALL_BINDIR}"
)

file(
  READ
  "${CMAKE_CURRENT_SOURCE_DIR}/yosys_lib_sources.json"
  yosys_lib_sources
)

function(add_yosys_library lib_name)
  string(JSON lib_sources_json GET "${yosys_lib_sources}" "${lib_name}")
  string(JSON lib_sources_len LENGTH "${lib_sources_json}")
  math(EXPR lib_sources_end "${lib_sources_len}-1")

  foreach(lib_sources_i RANGE ${lib_sources_end})
    string(JSON lib_source GET "${lib_sources_json}" "${lib_sources_i}")
    list(APPEND lib_sources "${CMAKE_CURRENT_SOURCE_DIR}/yosys/${lib_source}")
  endforeach()

  add_library(${lib_name} STATIC ${lib_sources})

  target_include_directories(
    ${lib_name}
    PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/yosys"
    "${CMAKE_CURRENT_SOURCE_DIR}/include"
  )

  target_compile_definitions(
    ${lib_name}
    PRIVATE
    _YOSYS_=1
  )

  target_compile_options(
    ${lib_name}
    PRIVATE
    "$<${gcc_like_cxx}:${gcc_like_compile_options}>"
  )

  set_target_properties(
    ${lib_name}
    PROPERTIES
    POSITION_INDEPENDENT_CODE ON
  )
endfunction()

file(STRINGS "${CMAKE_CURRENT_SOURCE_DIR}/yosys_lib_names.txt" yosys_lib_names)

foreach(lib_name ${yosys_lib_names})
  add_yosys_library(${lib_name})
endforeach()

target_sources(
  YosysKernel
  PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/lib/kernel/version_vts.cc"
)

find_package(BISON REQUIRED)
find_package(FLEX REQUIRED)

bison_target(
  rtlil_parser
  "${CMAKE_SOURCE_DIR}/yosys/frontends/rtlil/rtlil_parser.y"
  "${CMAKE_BINARY_DIR}/yosys/frontends/rtlil/rtlil_parser.tab.cc"
  DEFINES_FILE
  "${CMAKE_BINARY_DIR}/yosys/frontends/rtlil/rtlil_parser.tab.hh"
)

if(NOT DEFINED BISON_rtlil_parser_DEFINED)
  message(FATAL_ERROR "failed to generate rtlil parser")
endif()

flex_target(
  rtlil_lexer
  "${CMAKE_SOURCE_DIR}/yosys/frontends/rtlil/rtlil_lexer.l"
  "${CMAKE_BINARY_DIR}/yosys/frontends/rtlil/rtlil_lexer.cc"
  DEFINES_FILE
  "${CMAKE_BINARY_DIR}/yosys/frontends/rtlil/rtlil_lexer.hh"
)

if(NOT DEFINED FLEX_rtlil_lexer_DEFINED)
  message(FATAL_ERROR "failed to generate rtlil lexer")
endif()

add_flex_bison_dependency(rtlil_lexer rtlil_parser)

target_include_directories(
  YosysFrontendsRtlil
  PRIVATE
  "${CMAKE_CURRENT_BINARY_DIR}/yosys"
)

target_sources(
  YosysFrontendsRtlil
  PRIVATE
  ${BISON_rtlil_parser_OUTPUT_SOURCE}
  ${FLEX_rtlil_lexer_OUTPUTS}
)

bison_target(
  verilog_parser
  "${CMAKE_SOURCE_DIR}/yosys/frontends/verilog/verilog_parser.y"
  "${CMAKE_BINARY_DIR}/yosys/frontends/verilog/verilog_parser.tab.cc"
  DEFINES_FILE
  "${CMAKE_BINARY_DIR}/yosys/frontends/verilog/verilog_parser.tab.hh"
)

if(NOT DEFINED BISON_verilog_parser_DEFINED)
  message(FATAL_ERROR "failed to generate verilog parser")
endif()

flex_target(
  verilog_lexer
  "${CMAKE_SOURCE_DIR}/yosys/frontends/verilog/verilog_lexer.l"
  "${CMAKE_BINARY_DIR}/yosys/frontends/verilog/verilog_lexer.cc"
  DEFINES_FILE
  "${CMAKE_BINARY_DIR}/yosys/frontends/verilog/verilog_lexer.hh"
)

if(NOT DEFINED FLEX_verilog_lexer_DEFINED)
  message(FATAL_ERROR "failed to generate verilog lexer")
endif()

add_flex_bison_dependency(verilog_lexer verilog_parser)

target_include_directories(
  YosysFrontendsVerilog
  PRIVATE
  "${CMAKE_CURRENT_BINARY_DIR}/yosys"
)

target_sources(
  YosysFrontendsVerilog
  PRIVATE
  ${BISON_verilog_parser_OUTPUT_SOURCE}
  ${FLEX_verilog_lexer_OUTPUTS}
)

install(
  TARGETS ${yosys_lib_names}
  EXPORT vts-yosys-targets
  LIBRARY DESTINATION "${CMAKE_INSTALL_LIBDIR}"
  ARCHIVE DESTINATION "${CMAKE_INSTALL_LIBDIR}"
  RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}"
  INCLUDES DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
)

if(${VTS_YOSYS_BUILD_DYLIB})
  add_library(vts-yosys SHARED vts-yosys.cpp)

  target_link_libraries(
    vts-yosys
    PRIVATE
    "$<${gcc_like_linker}:-Wl,--whole-archive>"
    ${yosys_lib_names}
    "$<${gcc_like_linker}:-Wl,--no-whole-archive>"
  )

  install(
    TARGETS vts-yosys
    EXPORT vts-yosys-targets
    LIBRARY DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    ARCHIVE DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}"
    INCLUDES DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
  )
endif()
