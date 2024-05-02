#include "wrapper.h"

#include <circt/Conversion/HWToLLHD.h>
#include <mlir/Pass/Pass.h>

#include <stdio.h>

void it_works() {
  const unsigned i = 43;
  printf("%u\n", i);
  circt::createConvertHWToLLHDPass();
}

void simplify(const char *filename) {
  printf("simplifying %s\n", filename);
}
