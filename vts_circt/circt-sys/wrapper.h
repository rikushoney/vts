#pragma once

#include <circt-c/Dialect/Comb.h>
#include <circt-c/Dialect/HW.h>
#include <circt-c/Dialect/LLHD.h>
#include <circt-c/Dialect/Seq.h>
#include <circt-c/Dialect/Moore.h>

#ifdef __cplusplus
#define CIRCT_SYS_C_API extern "C"
#else
#define CIRCT_SYS_C_API
#endif

#ifndef CIRCT_SYS_WRAPPER_H
#define CIRCT_SYS_WRAPPER_H

CIRCT_SYS_C_API void it_works();
CIRCT_SYS_C_API void simplify(const char *filename);

#endif
