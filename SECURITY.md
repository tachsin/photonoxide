# Security policy

## Reporting a vulnerability

Please report a security problem privately, not in a public issue: use [GitHub's private vulnerability reporting](https://github.com/tachsin/photonoxide/security/advisories/new). Include what you found, how to reproduce it, and the version.

I read reports as they come in and reply as soon as I can. A fix goes out as a patch release, with a note in the changelog once it is safe to describe.

## What is covered

The latest release of the library, the Python package and the studio. photonoxide reads job files, mesh and mask files and Touchstone files, loads native libraries (OpenBLAS, oneMKL, MUMPS, CUDA and others) at run time, and the studio updates itself from GitHub releases; reports in any of those are in scope.

Wrong simulation results are not security problems: please open an ordinary [bug report](https://github.com/tachsin/photonoxide/issues/new/choose).
