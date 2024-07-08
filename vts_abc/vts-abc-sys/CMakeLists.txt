cmake_minimum_required(VERSION 3.20)

string(TIMESTAMP vts_abc_calver "%Y%m%d" UTC)

project(
  vts_abc
  VERSION 0.0.${vts_abc_calver}
  LANGUAGES C CXX
)

option(VTS_ABC_BUILD_DYLIB "Build VTS ABC shared library" ON)

set(vts_abc_required_c_standard 11)
set(vts_abc_required_cxx_standard 17)

set(gcc_like_c "$<COMPILE_LANG_AND_ID:C,AppleClang,Clang,GNU>")
set(gcc_like_cxx "$<COMPILE_LANG_AND_ID:CXX,AppleClang,Clang,GNU>")
set(gcc_like_compiler "$<OR:${gcc_like_c},${gcc_like_cxx}>")
set(gcc_like_linker "$<LINK_LANG_AND_ID:CXX,AppleClang,Clang,GNU>")

set(
  gcc_like_compile_options
  "-Wall"
  "-Wno-sign-compare"
  "-Wno-unused-but-set-variable"
  "-Wno-unused-function"
  "-Wno-write-strings"
)

set(CMAKE_C_STANDARD ${vts_abc_required_c_standard})
set(CMAKE_C_STANDARD_REQUIRED YES)
set(CMAKE_CXX_STANDARD ${vts_abc_required_cxx_standard})
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

add_library(vts-abc-static STATIC)

target_include_directories(
  vts-abc-static
  PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/abc/src"
)

target_compile_definitions(
  vts-abc-static
  PRIVATE
  ABC_USE_CUDD=1
  ABC_USE_STDINT_H=1
)

target_compile_options(
  vts-abc-static
  PRIVATE
  "$<${gcc_like_compiler}:${gcc_like_compile_options}>"
)

if(${VTS_ABC_BUILD_DYLIB})
  set_target_properties(
    vts-abc-static
    PROPERTIES
    POSITION_INDEPENDENT_CODE ON
  )
endif()

function(parse_sources_list sources_list_file sources_list_out)
  file(READ "${sources_list_file}" raw_sources_list)
  string(STRIP "${raw_sources_list}" sources_list)
  list(TRANSFORM sources_list PREPEND "${CMAKE_CURRENT_SOURCE_DIR}/abc/")
  set(${sources_list_out} ${sources_list} PARENT_SCOPE)
endfunction()

parse_sources_list(
  "${CMAKE_CURRENT_SOURCE_DIR}/abc_sources.txt"
  abc_sources
)

target_sources(vts-abc-static PRIVATE ${abc_sources})

if(${VTS_ABC_BUILD_DYLIB})
  add_library(vts-abc SHARED vts-abc.cpp)

  target_link_libraries(
    vts-abc
    PRIVATE
    "$<${gcc_like_linker}:-Wl,--whole-archive>"
    vts-abc-static
    "$<${gcc_like_linker}:-Wl,--no-whole-archive>"
  )
endif()
