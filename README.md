# Analysis-TMP

Implementation of real-time schedulability analysis algorithms using C++ type system: real-time tasks are specified as C++ types and the analysis is done at compile time.

## Dependencies


- Loki library for template metaprogramming: https://sourceforge.net/projects/loki-lib

  - Implementations have been tested with Loki-0.1.7

## Build and test

The schedulability analyses are implemented as header files and so no build needed for the library. The `tests` directory provides a Makefile to build the tests for the corresponding algorithms.

To build the tests, a local Loki library must exist at the path specified by the `LOKI_ROOT` environment variable. If `LOKI_ROOT` is not defined, it assumes Loki exists in the root directory of the project.

Build tests by running:

```bash
cd tests
make
```

Output binaries are placed in `tests/build` directory.
