# Installing the external libraries

photonoxide needs none of these: its own solvers are built in, and the default build, CI and the released app work with no library installed. An external library is optional, installed by you under its own licence, found and loaded at run time, and never redistributed with photonoxide ([the plan](plans/backends.md)). The studio's Libraries page shows the same guides beside what it finds on your machine, and `photonoxide libraries` prints it.

This page is written by `photonoxide libraries --write docs/libraries.md` from the guides in the program; a test fails when it is out of date.

**Checked** means the command was run on a clean machine, one of GitHub's runners, and photonoxide then found the library: the Libraries workflow (`.github/workflows/libraries.yml`) does that again every week and on every change to the guides, and fails when a method stops working. What it can't check is said with each method: the runners have no GPU, so NVIDIA's libraries are found and loaded there but their backends' smoke tests don't run; winget's installers are several gigabytes and ask for elevation, so only the index is checked for the packages; and there is no Intel Mac among the runners.

## Which library installs where

| Library | Backends | Windows | Linux | macOS |
|---|---|---|---|---|
| [oneMKL](#onemkl) | `pardiso`, `photonoxide-mkl` | winget, conda-forge ✓, pip ✓ | conda-forge ✓, pip ✓, apt ✓ | conda-forge |
| [CUDA runtime](#cuda-runtime) | — | winget, conda-forge ✓, pip ✓ | conda-forge ✓, pip ✓ | see below |
| [cuSPARSE](#cusparse) | `cusparse` | winget, conda-forge ✓, pip ✓ | conda-forge ✓, pip ✓ | see below |
| [cuDSS](#cudss) | `cudss` | conda-forge ✓, pip ✓ | conda-forge ✓, pip ✓ | see below |
| [SuperLU](#superlu) | `superlu` | see below | conda-forge ✓, apt ✓ | conda-forge ✓ |
| [MUMPS](#mumps) | `mumps`, `mumps-blr` | conda-forge ✓ | conda-forge ✓, apt ✓ | conda-forge ✓ |
| [Accelerate](#accelerate) | `accelerate`, `photonoxide-accelerate` | see below | see below | see below |
| [OpenBLAS](#openblas) | `photonoxide-openblas` | conda-forge ✓ | conda-forge ✓, apt ✓ | conda-forge ✓ |
| [AMD AOCL](#amd-aocl) | none yet (#186) | see below | see below | see below |
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

Intel's oneAPI Math Kernel Library, through its single library mkl_rt: PARDISO, its sparse direct solver, and its BLAS and LAPACK as the dense kernels of photonoxide's own solver's fronts.

- **Licence:** [Intel Simplified Software License (October 2022)](https://cdrdv2-public.intel.com/749362/intel-simplified-license-software-october-2022.pdf). You accept it by installing.
- **The vendor's download:** <https://www.intel.com/content/www/us/en/developer/tools/oneapi/onemkl-download.html>
- **Backends:** `pardiso`, `photonoxide-mkl`.

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

### SuperLU

A supernodal sparse LU with partial pivoting (Demmel, Eisenstat, Gilbert, Li, Liu 1999), the sequential library, releases 5 to 7.

- **Licence:** [BSD-3-Clause](https://github.com/xiaoyeli/superlu/blob/master/License.txt). You accept it by installing.
- **The vendor's download:** <https://portal.nersc.gov/project/sparse/superlu/>
- **Backends:** `superlu`.

- **conda-forge** (Linux, macOS):

  ```sh
  conda install -c conda-forge superlu
  ```

  Into the active conda environment: start photonoxide from it.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed superlu 7.0.1; found SuperLU 7.0.0 by its file's name, and the superlu backend after its smoke test and its tests.
  - Checked on macOS (macos-latest (Apple silicon)), 2026-10-09: installed superlu 7.0.1; found SuperLU 7.0.0 by its file's name, and the superlu backend after its smoke test and its tests.

- **apt** (Linux):

  ```sh
  sudo apt-get install libsuperlu-dev
  ```

  Debian's and Ubuntu's package, older than conda-forge's: 6.0.1 on Ubuntu 24.04, 5.3.0 on 22.04.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed libsuperlu-dev 6.0.1; found SuperLU 6.0.1 by its file's name, and the superlu backend after its smoke test and its tests.
  - Checked on Linux (ubuntu-22.04), 2026-10-09: installed libsuperlu-dev 5.3.0; found SuperLU 5.3.0 by its file's name, and the superlu backend after its smoke test and its tests.

- **windows:** conda-forge has no superlu for Windows (the install fails with PackagesNotFoundInChannelsError, 2026-10-09), and no package there names SuperLU's library by its release, which is how photonoxide tells the releases apart: photonoxide's own solvers run there.

### MUMPS

A multifrontal sparse direct solver (Amestoy, Duff, L'Excellent, Koster 2001; Amestoy, Buttari, L'Excellent, Mary 2019), its sequential build, and its block low-rank factorization (Amestoy et al. 2015) as a backend of its own, lossy to a tolerance and refined. MUMPS asks that work using it cite it.

- **Licence:** [CeCILL-C](https://cecill.info/licences/Licence_CeCILL-C_V1-en.html). You accept it by installing.
- **The vendor's download:** <https://mumps-solver.org/>
- **Backends:** `mumps`, `mumps-blr`.

- **conda-forge** (Windows, Linux, macOS):

  ```sh
  conda install -c conda-forge mumps-seq
  ```

  The sequential build, into the active conda environment: start photonoxide from it. On Windows it brings conda-forge's mkl with it, and so oneMKL's PARDISO too.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed mumps-seq 5.8.2; found MUMPS 5.8.2, and the mumps backend after its smoke test and its tests.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed mumps-seq 5.8.2, mkl 2026.1.0; found MUMPS 5.8.2, and the mumps backend after its smoke test and its tests.
  - Checked on macOS (macos-latest (Apple silicon)), 2026-10-09: installed mumps-seq 5.8.2; found MUMPS 5.8.2, and the mumps backend after its smoke test and its tests.

- **apt** (Linux):

  ```sh
  sudo apt-get install libmumps-seq-dev
  ```

  Debian's and Ubuntu's package, older than conda-forge's (5.6.2 on Ubuntu 24.04, 5.4.1 on 22.04): both are releases photonoxide knows.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed libmumps-seq-dev 5.6.2; found MUMPS 5.6.2, and the mumps backend after its smoke test and its tests.
  - Checked on Linux (ubuntu-22.04), 2026-10-09: installed libmumps-seq-dev 5.4.1; found MUMPS 5.4.1, and the mumps backend after its smoke test and its tests.

### Accelerate

Apple's Accelerate framework, its sparse direct solvers, and its BLAS and LAPACK as the dense kernels of photonoxide's own solver's fronts: part of macOS, nothing to install. Complex LU from macOS 15.5, complex symmetric L D Lᵀ from macOS 26.

- **Licence:** [Part of macOS, under Apple's software licence agreement for it](https://www.apple.com/legal/sla/). You accept it by installing.
- **The vendor's download:** <https://developer.apple.com/documentation/accelerate/sparse_solvers>
- **Backends:** `accelerate`, `photonoxide-accelerate`.

- **macos:** Built in: nothing to install. Its complex LU needs macOS 15.5, its complex symmetric L D Lᵀ macOS 26; on an older macOS photonoxide's own solvers run.
- **windows:** Accelerate is part of macOS: photonoxide's own solvers run here.
- **linux:** Accelerate is part of macOS: photonoxide's own solvers run here.

### OpenBLAS

An open BLAS and LAPACK: the dense kernels of photonoxide's own solver's fronts, in place of faer's.

- **Licence:** [BSD-3-Clause](https://github.com/OpenMathLib/OpenBLAS/blob/develop/LICENSE). You accept it by installing.
- **The vendor's download:** <https://github.com/OpenMathLib/OpenBLAS/releases>
- **Backends:** `photonoxide-openblas`.

- **conda-forge** (Windows, Linux, macOS):

  ```sh
  conda install -c conda-forge openblas
  ```

  Into the active conda environment: start photonoxide from it. conda-forge's macOS build has no openblas_set_num_threads_local, and photonoxide holds it to one thread.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed openblas 0.3.34; found OpenBLAS 0.3.34, and the photonoxide-openblas backend after its smoke test and its tests.
  - Checked on Windows (windows-2025-vs2026), 2026-10-09: installed openblas 0.3.34; found OpenBLAS 0.3.34, and the photonoxide-openblas backend after its smoke test and its tests.
  - Checked on macOS (macos-latest (Apple silicon)), 2026-10-09: installed openblas 0.3.34; found OpenBLAS 0.3.34, and the photonoxide-openblas backend after its smoke test and its tests, on one thread.

- **apt** (Linux):

  ```sh
  sudo apt-get install libopenblas-dev
  ```

  Debian's and Ubuntu's package: 0.3.26 on Ubuntu 24.04, 0.3.20 on 22.04. Before OpenBLAS 0.3.27 its threads can't be set for each call, and photonoxide holds it to one.

  - Checked on Linux (ubuntu-24.04), 2026-10-09: installed libopenblas-dev 0.3.26; found OpenBLAS 0.3.26, and the photonoxide-openblas backend after its smoke test and its tests, on one thread.
  - Checked on Linux (ubuntu-22.04), 2026-10-09: installed libopenblas-dev 0.3.20; found OpenBLAS 0.3.20, and the photonoxide-openblas backend after its smoke test and its tests, on one thread.

## The libraries without a backend yet

How each installs, for the issue that adds its backend. photonoxide doesn't look for them yet.

### AMD AOCL

AMD's BLIS, libFLAME and AOCL-Sparse: dense kernels and iterative solvers on AMD processors. Its backend: [#186](https://github.com/tachsin/photonoxide/issues/186).

- **Licence:** BSD-3-Clause and MIT, by component (to confirm in #186).
- **Home:** <https://www.amd.com/en/developer/aocl.html>

- **windows:** AMD's installer, from its page: no package manager has it, and it wasn't run here.
- **linux:** AMD's packages or Spack, from its page: not run here.
- **macos:** AMD has no macOS build.

### Arm Performance Libraries

Arm's BLAS and LAPACK: dense kernels on Arm processors. Its backend: [#186](https://github.com/tachsin/photonoxide/issues/186).

- **Licence:** Arm's terms, free of charge (to confirm in #186).
- **Home:** <https://developer.arm.com/>

- **windows:** Arm's installer, for Windows on Arm: not run here.
- **linux:** Arm's packages, for AArch64: not run here.
- **macos:** Arm's installer, for Apple silicon: not run here.

## New releases

Every Monday the Releases workflow (`.github/workflows/releases.yml`) runs `photonoxide libraries --releases`. It reads each library's newest release where it is published: PyPI's and anaconda.org's APIs for the wheels and conda-forge's packages, NVIDIA's redistributables' manifests for CUDA and cuDSS, the GitHub releases of SuperLU and OpenBLAS, MUMPS's download page, and, for Accelerate, the macOS of GitHub's runner images. It sets each beside the version checked above and the releases photonoxide-native accepts, with a link to the release's notes, and keeps that table in one issue, [New releases of external libraries](https://github.com/tachsin/photonoxide/issues?q=is%3Aissue+in%3Atitle+%22New+releases+of+external+libraries%22), commenting there when a release appears.

A new release isn't supported until it has been checked:

1. **Read its release notes** for what photonoxide calls: MUMPS's `ZMUMPS_STRUC_C` and SuperLU's options change between releases, and NVIDIA's, oneMKL's and OpenBLAS's file names carry a major release.
2. **Outside what photonoxide-native accepts,** it is refused until its support is added: the release's layout in `native/src/mumps.rs` (from its `zmumps_c.h`), its major release in `native/src/superlu.rs`, its file names in `native/src/nvidia.rs`.
3. **Install it on clean machines:** the Libraries workflow on a branch (`gh workflow run libraries.yml --ref <branch>`) installs each method, asks photonoxide what it finds, and runs each backend's tests with its library required.
4. **Check it against photonoxide's own solvers** on a machine that has it (and a GPU, for NVIDIA's): the backend's tests (`cargo test -p photonoxide-native --release --test <backend>`, with `PHOTONOXIDE_REQUIRE_<LIBRARY>` set where the test has one) and the benchmark's problems (`photonoxide bench --tier standard --backends <backend>`), whose answers are checked.
5. **Then record it:** its `Checked` entries in `studio/src-tauri/src/libraries.rs`, and `photonoxide libraries --write docs/libraries.md`. Only then do the guides call it checked.

## Traps

- **Debian's and Ubuntu's oneMKL is MKL 2020.4.** `apt` installs a library six years older than Intel's, conda-forge's or pip's. photonoxide finds it and its PARDISO passes the smoke test; prefer the others for a current library.
- **A conda environment must be the active one** when photonoxide starts: it is found by `CONDA_PREFIX`.
- **pip installs into the Python that runs it.** photonoxide looks in the active virtual environment and in the Python on `PATH`; a wheel installed into another Python isn't found unless the library's variable points at it.
- **NVIDIA's wheels for CUDA 13 keep their libraries in two folders** (`nvidia/cu13/bin` and `bin/x86_64` on Windows). photonoxide adds the second to the folders Windows searches when cuDSS needs it.
- **conda-forge's mumps-seq on Windows brings oneMKL with it,** and so the `pardiso` backend.
- **Every NVIDIA library needs NVIDIA's driver and a CUDA GPU** to do anything: found and loaded isn't yet a backend.
