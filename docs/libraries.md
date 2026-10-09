# Installing the external libraries

photonoxide needs none of these: its own solvers are built in, and the default build, CI and the released app work with no library installed. An external library is optional, installed by you under its own licence, found and loaded at run time, and never redistributed with photonoxide ([the plan](plans/backends.md)). The studio's Libraries page shows the same guides beside what it finds on your machine, and `photonoxide libraries` prints it.

This page is written by `photonoxide libraries --write docs/libraries.md` from the guides in the program; a test fails when it is out of date.

**Checked** means the command was run on a clean machine, one of GitHub's runners, and photonoxide then found the library: the Libraries workflow (`.github/workflows/libraries.yml`) does that again every week and on every change to the guides, and fails when a method stops working. What it can't check is said with each method: the runners have no GPU, so NVIDIA's libraries are found and loaded there but their backends' smoke tests don't run; winget's installers are several gigabytes and ask for elevation, so only the index is checked for the packages; and there is no Intel Mac among the runners.

## Which library installs where

| Library | Backends | Windows | Linux | macOS |
|---|---|---|---|---|
| [oneMKL](#onemkl) | `pardiso` | winget, conda-forge ✓, pip ✓ | conda-forge ✓, pip ✓, apt ✓ | conda-forge |
| [CUDA runtime](#cuda-runtime) | — | winget, conda-forge ✓, pip ✓ | conda-forge ✓, pip ✓ | see below |
| [cuSPARSE](#cusparse) | `cusparse` | winget, conda-forge ✓, pip ✓ | conda-forge ✓, pip ✓ | see below |
| [cuDSS](#cudss) | `cudss` | conda-forge ✓, pip ✓ | conda-forge ✓, pip ✓ | see below |
| [Accelerate](#accelerate) | `accelerate` | see below | see below | see below |
| [MUMPS](#mumps) | none yet (#176) | conda-forge ✓ | conda-forge ✓, apt ✓ | conda-forge ✓ |
| [SuperLU](#superlu) | none yet (#177) | see below | conda-forge ✓, apt ✓ | conda-forge ✓ |
| [OpenBLAS](#openblas) | none yet (#186) | conda-forge ✓ | conda-forge ✓, apt ✓ | conda-forge ✓ |
| [AMD AOCL](#amd-aocl) | none yet (#186) | see below | see below | see below |
| [Apple Accelerate](#apple-accelerate) | none yet (#186) | see below | see below | see below |
| [Arm Performance Libraries](#arm-performance-libraries) | none yet (#186) | see below | see below | see below |

✓: run on a clean machine and found. A manager without it is listed by its package index alone.

## Where photonoxide looks

For each library, in this order, and the first file that loads and passes its check is used:

1. the library's own variable, naming the file or its folder (`PHOTONOXIDE_MKL`, say), then the vendor's (`MKLROOT`, `CUDA_PATH`);
2. the active conda environment (`CONDA_PREFIX`): start photonoxide from it;
3. the vendor's default install folders;
4. Python's package folders, for the vendors' wheels: the active virtual environment's, the Python on `PATH`, the user's and the system's. Nothing needs Python to load them;
5. the system's library paths.

## The libraries photonoxide has a backend for

### oneMKL

Intel's oneAPI Math Kernel Library, through its single library mkl_rt: PARDISO, its sparse direct solver.

- **Licence:** [Intel Simplified Software License (October 2022)](https://cdrdv2-public.intel.com/749362/intel-simplified-license-software-october-2022.pdf). You accept it by installing.
- **The vendor's download:** <https://www.intel.com/content/www/us/en/developer/tools/oneapi/onemkl-download.html>
- **Backends:** `pardiso`.

- **winget** (Windows):

  ```sh
  winget install --id Intel.oneMKL --exact
  ```

  Intel's installer, into Program Files\Intel\oneAPI, where photonoxide looks.

  Not run on a clean machine: the installer is several gigabytes and asks for elevation. The workflow checks that winget's index still has the package.

- **conda-forge** (Windows, Linux, macOS):

  ```sh
  conda install -c conda-forge mkl
  ```

  Into the active conda environment: start photonoxide from it, so $CONDA_PREFIX points there. On macOS, Intel Macs only.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed mkl 2026.1.0; found oneMKL 2026.1, and the pardiso backend after its smoke test.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed mkl 2026.1.0; found oneMKL 2026.1, and the pardiso backend after its smoke test.

- **pip** (Windows, Linux):

  ```sh
  pip install mkl
  ```

  The wheel puts mkl_rt beside Python (Library\bin on Windows, lib elsewhere), where photonoxide looks: the Python on PATH, or the active virtual environment's.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed mkl 2026.1.0; found oneMKL 2026.1, and the pardiso backend after its smoke test.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed mkl 2026.1.0; found oneMKL 2026.1, and the pardiso backend after its smoke test.

- **apt** (Linux):

  ```sh
  sudo apt-get install libmkl-rt
  ```

  Debian's and Ubuntu's package: MKL 2020.4, from 2020, where the other methods give this year's. photonoxide finds it in the system's library path.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed libmkl-rt 2020.4.304-4; found oneMKL 2020.4, and the pardiso backend after its smoke test.
  - Checked on Linux (ubuntu-22.04), 2026-10-09: installed libmkl-rt 2020.4.304-2ubuntu3; found oneMKL 2020.4, and the pardiso backend after its smoke test.

- **macos-aarch64:** oneMKL has no build for Apple silicon: photonoxide's own solvers run there.

### CUDA runtime

NVIDIA's CUDA runtime: the GPU backends' memory and devices. It needs NVIDIA's driver and a CUDA GPU.

- **Licence:** [NVIDIA CUDA Toolkit End User License Agreement](https://docs.nvidia.com/cuda/eula/index.html). You accept it by installing.
- **The vendor's download:** <https://developer.nvidia.com/cuda-downloads>

- **winget** (Windows):

  ```sh
  winget install --id Nvidia.CUDA --exact
  ```

  The whole CUDA toolkit (several GB), with cuSPARSE; $CUDA_PATH points at it.

  Not run on a clean machine: the installer is several gigabytes and asks for elevation. The workflow checks that winget's index still has the package.

- **conda-forge** (Windows, Linux):

  ```sh
  conda install -c conda-forge cuda-cudart libcusparse
  ```

  The runtime and cuSPARSE alone, into the active conda environment: start photonoxide from it.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed cuda-cudart 13.4.92, libcusparse 12.8.6.72; found the CUDA runtime 13.4, which reports no GPU there.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed cuda-cudart 13.4.92, libcusparse 12.8.6.72; found the CUDA runtime's file, which reports no GPU there.

- **pip** (Windows, Linux):

  ```sh
  pip install nvidia-cuda-runtime nvidia-cusparse
  ```

  NVIDIA's CUDA 13 wheels, found in site-packages/nvidia; nothing needs Python to load them.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed nvidia-cuda-runtime 13.4.92, nvidia-cusparse 12.8.6.72; found the CUDA runtime 13.4, which reports no GPU there.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed nvidia-cuda-runtime 13.4.92, nvidia-cusparse 12.8.6.72; found the CUDA runtime's file, which reports no GPU there.

- **macos:** NVIDIA's libraries have no macOS build: photonoxide's own solvers run there.

### cuSPARSE

The CUDA toolkit's sparse kernels: photonoxide's QMR with ILU(0) run on the GPU.

- **Licence:** [NVIDIA CUDA Toolkit End User License Agreement](https://docs.nvidia.com/cuda/eula/index.html). You accept it by installing.
- **The vendor's download:** <https://developer.nvidia.com/cuda-downloads>
- **Backends:** `cusparse`.
- **Needs:** CUDA runtime.

- **winget** (Windows):

  ```sh
  winget install --id Nvidia.CUDA --exact
  ```

  The whole CUDA toolkit (several GB), with the runtime; $CUDA_PATH points at it.

  Not run on a clean machine: the installer is several gigabytes and asks for elevation. The workflow checks that winget's index still has the package.

- **conda-forge** (Windows, Linux):

  ```sh
  conda install -c conda-forge cuda-cudart libcusparse
  ```

  Into the active conda environment: start photonoxide from it.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed cuda-cudart 13.4.92, libcusparse 12.8.6.72; found cuSPARSE 12.8.6, loaded; the runner has no GPU, so the backend's smoke test couldn't run.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed cuda-cudart 13.4.92, libcusparse 12.8.6.72; found cuSPARSE 12.8.6, loaded; the runner has no GPU, so the backend's smoke test couldn't run.

- **pip** (Windows, Linux):

  ```sh
  pip install nvidia-cuda-runtime nvidia-cusparse
  ```

  NVIDIA's CUDA 13 wheels, found in site-packages/nvidia.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed nvidia-cuda-runtime 13.4.92, nvidia-cusparse 12.8.6.72; found cuSPARSE 12.8.6, loaded; the runner has no GPU, so the backend's smoke test couldn't run.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed nvidia-cuda-runtime 13.4.92, nvidia-cusparse 12.8.6.72; found cuSPARSE 12.8.6, loaded; the runner has no GPU, so the backend's smoke test couldn't run.

- **macos:** NVIDIA's libraries have no macOS build: photonoxide's own solvers run there.

### cuDSS

NVIDIA's sparse direct solver on the GPU.

- **Licence:** [NVIDIA cuDSS Software License Agreement](https://docs.nvidia.com/cuda/cudss/license.html). You accept it by installing.
- **The vendor's download:** <https://developer.nvidia.com/cudss-downloads>
- **Backends:** `cudss`.
- **Needs:** CUDA runtime.

- **conda-forge** (Windows, Linux):

  ```sh
  conda install -c conda-forge libcudss
  ```

  Into the active conda environment: start photonoxide from it.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed libcudss 0.8.0.10; found cuDSS 0.8.0, loaded; the runner has no GPU, so the backend's smoke test couldn't run.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed libcudss 0.8.0.10; found cuDSS 0.8.0, loaded; the runner has no GPU, so the backend's smoke test couldn't run.

- **pip** (Windows, Linux):

  ```sh
  pip install nvidia-cudss-cu13
  ```

  The CUDA 13 build (nvidia-cudss-cu12 for CUDA 12), found in site-packages/nvidia.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed nvidia-cudss-cu13 0.8.0.10; found cuDSS 0.8.0, loaded; the runner has no GPU, so the backend's smoke test couldn't run.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed nvidia-cudss-cu13 0.8.0.10; found cuDSS 0.8.0, loaded; the runner has no GPU, so the backend's smoke test couldn't run.

- **macos:** NVIDIA's libraries have no macOS build: photonoxide's own solvers run there.

### Accelerate

Apple's Accelerate framework, its sparse direct solvers: part of macOS, nothing to install. Complex LU from macOS 15.5, complex symmetric L D Lᵀ from macOS 26.

- **Licence:** [Part of macOS, under Apple's software licence agreement for it](https://www.apple.com/legal/sla/). You accept it by installing.
- **The vendor's download:** <https://developer.apple.com/documentation/accelerate/sparse_solvers>
- **Backends:** `accelerate`.

- **macos:** Built in: nothing to install. Its complex LU needs macOS 15.5, its complex symmetric L D Lᵀ macOS 26; on an older macOS photonoxide's own solvers run.
- **windows:** Accelerate is part of macOS: photonoxide's own solvers run here.
- **linux:** Accelerate is part of macOS: photonoxide's own solvers run here.

## The libraries without a backend yet

How each installs, for the issue that adds its backend. photonoxide doesn't look for them yet.

### MUMPS

A multifrontal sparse direct solver, with block low-rank compression: a direct backend. The sequential build. Its backend: [#176](https://github.com/tachsin/photonoxide/issues/176).

- **Licence:** CeCILL-C.
- **Home:** <https://mumps-solver.org/>

- **conda-forge** (Windows, Linux, macOS):

  ```sh
  conda install -c conda-forge mumps-seq
  ```

  The sequential build, into the active conda environment. On Windows it brings conda-forge's mkl with it, and so oneMKL's PARDISO too.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed mumps-seq 5.8.2; found the package's library is there; photonoxide has no backend to load it with yet.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed mumps-seq 5.8.2, mkl 2026.1.0; found the package's library is there; photonoxide has no backend to load it with yet.
  - Checked on macOS (macos-latest (Apple silicon)), 2026-10-09: installed mumps-seq 5.8.2; found the package's library is there; photonoxide has no backend to load it with yet.

- **apt** (Linux):

  ```sh
  sudo apt-get install libmumps-seq-dev
  ```

  Debian's and Ubuntu's package, older than conda-forge's: 5.6.2 on Ubuntu 24.04, 5.4.1 on 22.04.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed libmumps-seq-dev 5.6.2; found the package's library is there; photonoxide has no backend to load it with yet.
  - Checked on Linux (ubuntu-22.04), 2026-10-09: installed libmumps-seq-dev 5.4.1; found the package's library is there; photonoxide has no backend to load it with yet.

### SuperLU

A supernodal sparse direct solver with partial pivoting: a direct backend. Its backend: [#177](https://github.com/tachsin/photonoxide/issues/177).

- **Licence:** BSD-3-Clause.
- **Home:** <https://portal.nersc.gov/project/sparse/superlu/>

- **conda-forge** (Linux, macOS):

  ```sh
  conda install -c conda-forge superlu
  ```

  Into the active conda environment.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed superlu 7.0.1; found the package's library is there; photonoxide has no backend to load it with yet.
  - Checked on macOS (macos-latest (Apple silicon)), 2026-10-09: installed superlu 7.0.1; found the package's library is there; photonoxide has no backend to load it with yet.

- **apt** (Linux):

  ```sh
  sudo apt-get install libsuperlu-dev
  ```

  Debian's and Ubuntu's package, older than conda-forge's: 6.0.1 on Ubuntu 24.04, 5.3.0 on 22.04.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed libsuperlu-dev 6.0.1; found the package's library is there; photonoxide has no backend to load it with yet.
  - Checked on Linux (ubuntu-22.04), 2026-10-09: installed libsuperlu-dev 5.3.0; found the package's library is there; photonoxide has no backend to load it with yet.

- **windows:** conda-forge has no superlu for Windows (the install fails with PackagesNotFoundInChannelsError, 2026-10-09). vcpkg builds it from source, which wasn't run here.

### OpenBLAS

An open BLAS and LAPACK: dense kernels for the multifrontal fronts, where no vendor's library is installed. Its backend: [#186](https://github.com/tachsin/photonoxide/issues/186).

- **Licence:** BSD-3-Clause.
- **Home:** <https://www.openblas.net/>

- **conda-forge** (Windows, Linux, macOS):

  ```sh
  conda install -c conda-forge openblas
  ```

  Into the active conda environment.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed openblas 0.3.34; found the package's library is there; photonoxide has no backend to load it with yet.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed openblas 0.3.34; found the package's library is there; photonoxide has no backend to load it with yet.
  - Checked on macOS (macos-latest (Apple silicon)), 2026-10-09: installed openblas 0.3.34; found the package's library is there; photonoxide has no backend to load it with yet.

- **apt** (Linux):

  ```sh
  sudo apt-get install libopenblas-dev
  ```

  Debian's and Ubuntu's package: 0.3.26 on Ubuntu 24.04, 0.3.20 on 22.04.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed libopenblas-dev 0.3.26; found the package's library is there; photonoxide has no backend to load it with yet.
  - Checked on Linux (ubuntu-22.04), 2026-10-09: installed libopenblas-dev 0.3.20; found the package's library is there; photonoxide has no backend to load it with yet.

### AMD AOCL

AMD's BLIS, libFLAME and AOCL-Sparse: dense kernels and iterative solvers on AMD processors. Its backend: [#186](https://github.com/tachsin/photonoxide/issues/186).

- **Licence:** BSD-3-Clause and MIT, by component (to confirm in #186).
- **Home:** <https://www.amd.com/en/developer/aocl.html>

- **windows:** AMD's installer, from its page: no package manager has it, and it wasn't run here.
- **linux:** AMD's packages or Spack, from its page: not run here.
- **macos:** AMD has no macOS build.

### Apple Accelerate

macOS's own BLAS and LAPACK: dense kernels on a Mac. (Its sparse solvers are the accelerate backend.) Its backend: [#186](https://github.com/tachsin/photonoxide/issues/186).

- **Licence:** part of macOS.
- **Home:** <https://developer.apple.com/documentation/accelerate>

- **macos:** Built in: nothing to install.
- **windows:** A macOS framework.
- **linux:** A macOS framework.

### Arm Performance Libraries

Arm's BLAS and LAPACK: dense kernels on Arm processors. Its backend: [#186](https://github.com/tachsin/photonoxide/issues/186).

- **Licence:** Arm's terms, free of charge (to confirm in #186).
- **Home:** <https://developer.arm.com/>

- **windows:** Arm's installer, for Windows on Arm: not run here.
- **linux:** Arm's packages, for AArch64: not run here.
- **macos:** Arm's installer, for Apple silicon: not run here.

## Traps

- **Debian's and Ubuntu's oneMKL is MKL 2020.4.** `apt` installs a library six years older than Intel's, conda-forge's or pip's. photonoxide finds it and its PARDISO passes the smoke test; prefer the others for a current library.
- **A conda environment must be the active one** when photonoxide starts: it is found by `CONDA_PREFIX`.
- **pip installs into the Python that runs it.** photonoxide looks in the active virtual environment and in the Python on `PATH`; a wheel installed into another Python isn't found unless the library's variable points at it.
- **NVIDIA's wheels for CUDA 13 keep their libraries in two folders** (`nvidia/cu13/bin` and `bin/x86_64` on Windows). photonoxide adds the second to the folders Windows searches when cuDSS needs it.
- **conda-forge's mumps-seq on Windows brings oneMKL with it,** and so the `pardiso` backend.
- **Every NVIDIA library needs NVIDIA's driver and a CUDA GPU** to do anything: found and loaded isn't yet a backend.
