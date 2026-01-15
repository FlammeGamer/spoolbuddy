#!/bin/bash

cd build
# Run all tests via CTest
make test
# Or with verbose output on failure
ctest --output-on-failure
# Run test executables directly
./tests/unit_tests
./tests/integration_tests
cd ..
  