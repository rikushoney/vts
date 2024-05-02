#pragma once

#ifdef __cplusplus
#define CIRCT_SYS_C_API extern "C"
#else
#define CIRCT_SYS_C_API
#endif

#ifndef CIRCT_SYS_WRAPPER_H
#define CIRCT_SYS_WRAPPER_H

CIRCT_SYS_C_API void it_works();

#endif
