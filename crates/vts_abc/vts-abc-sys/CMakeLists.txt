cmake_minimum_required(VERSION 3.20)

string(TIMESTAMP vts_abc_calver "%Y%m%d" UTC)

project(
  vts_abc
  VERSION 0.0.${vts_abc_calver}
  LANGUAGES C CXX
)

option(VTS_ABC_BUILD_DYLIB "Build VTS ABC shared library" OFF)

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

file(READ "${CMAKE_CURRENT_SOURCE_DIR}/abc_lib_sources.json" abc_lib_sources)

function(add_abc_library lib_name)
  string(JSON lib_sources_json GET "${abc_lib_sources}" "${lib_name}")
  string(JSON lib_sources_len LENGTH "${lib_sources_json}")
  math(EXPR lib_sources_end "${lib_sources_len}-1")

  foreach(lib_sources_i RANGE ${lib_sources_end})
    string(JSON lib_source GET "${lib_sources_json}" "${lib_sources_i}")
    list(APPEND lib_sources "${CMAKE_CURRENT_SOURCE_DIR}/abc/${lib_source}")
  endforeach()

  add_library(${lib_name} STATIC ${lib_sources})

  target_include_directories(
    ${lib_name}
    PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/abc/src"
  )

  target_compile_definitions(
    ${lib_name}
    PRIVATE
    ABC_USE_CUDD=1
    ABC_USE_NO_HISTORY=1
    ABC_USE_STDINT_H=1
  )

  target_compile_options(
    ${lib_name}
    PRIVATE
    "$<${gcc_like_compiler}:${gcc_like_compile_options}>"
  )

  set_target_properties(
    ${lib_name}
    PROPERTIES
    POSITION_INDEPENDENT_CODE ON
  )
endfunction()

file(STRINGS "${CMAKE_CURRENT_SOURCE_DIR}/abc_lib_names.txt" abc_lib_names)

foreach(lib_name ${abc_lib_names})
  add_abc_library(${lib_name})
endforeach()

install(
  TARGETS ${abc_lib_names}
  EXPORT vts-abc-targets
  LIBRARY DESTINATION "${CMAKE_INSTALL_LIBDIR}"
  ARCHIVE DESTINATION "${CMAKE_INSTALL_LIBDIR}"
  RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}"
  INCLUDES DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
)

if(${VTS_ABC_BUILD_DYLIB})
  add_library(vts-abc SHARED vts-abc.cpp)

  target_link_libraries(
    vts-abc
    PRIVATE
    "$<${gcc_like_linker}:-Wl,--whole-archive>"
    ${abc_lib_names}
    "$<${gcc_like_linker}:-Wl,--no-whole-archive>"
  )

  install(
    TARGETS vts-abc
    EXPORT vts-abc-targets
    LIBRARY DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    ARCHIVE DESTINATION "${CMAKE_INSTALL_LIBDIR}"
    RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}"
    INCLUDES DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}"
  )
endif()
