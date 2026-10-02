# Changelog

All notable changes to photonoxide are documented in this file, generated from the pull request titles by [release-plz](https://release-plz.dev/).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and photonoxide adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1](https://github.com/tachsin/photonoxide/compare/v0.3.0...v0.3.1) - 2026-10-02

### <!-- 0 -->Added

- *(studio)* the studio as a workspace, with examples inside and updates by one click ([#52](https://github.com/tachsin/photonoxide/pull/52))

## [0.3.0](https://github.com/tachsin/photonoxide/compare/v0.2.0...v0.3.0) - 2026-10-02

### <!-- 0 -->Added

- [**breaking**] make the studio a Tauri app with a 3D view, and the photonoxide program ([#36](https://github.com/tachsin/photonoxide/pull/36))
- open the studio's start page with a bare photonoxide, and release binaries ([#38](https://github.com/tachsin/photonoxide/pull/38))
- add a multilayer stack's reflection and transmission of a plane wave ([#40](https://github.com/tachsin/photonoxide/pull/40))
- add 2D FDFD with stretched-coordinate PMLs and its exact power flux ([#41](https://github.com/tachsin/photonoxide/pull/41))
- add 2D FDFD ports: the grid's own modes, one-way sources and a reciprocal S-matrix ([#42](https://github.com/tachsin/photonoxide/pull/42))
- reuse an FDFD matrix's symbolic analysis across a sweep ([#43](https://github.com/tachsin/photonoxide/pull/43))
- add an fdfd job: a device on one layer by 2D FDFD with ports, live in the studio ([#44](https://github.com/tachsin/photonoxide/pull/44))
- add adjoint gradients for 2D FDFD, checked against finite differences ([#45](https://github.com/tachsin/photonoxide/pull/45))
- add Hadley's high-accuracy interface and corner equations as a full-vector mode solver ([#47](https://github.com/tachsin/photonoxide/pull/47))
- add 3D FDFD on the Yee grid with stretched-coordinate PMLs ([#46](https://github.com/tachsin/photonoxide/pull/46))
- add a QMR iterative solver for 3D FDFD, on the curl-curl operator or Shin and Fan's ([#48](https://github.com/tachsin/photonoxide/pull/48))

### <!-- 1 -->Fixed

- *(fdfd)* put Shin and Fan's ε⁻¹ at the nodes, inside the gradient ([#50](https://github.com/tachsin/photonoxide/pull/50))

### <!-- 4 -->Documentation

- reshape the roadmap around components, circuits and active photonics ([#49](https://github.com/tachsin/photonoxide/pull/49))
- mark 0.2 and 0.3 done in the roadmap ([#51](https://github.com/tachsin/photonoxide/pull/51))

## [0.2.0](https://github.com/tachsin/photonoxide/compare/v0.1.1...v0.2.0) - 2026-10-02

### <!-- 0 -->Added

- add the exact TE and TM modes of three-layer slabs ([#14](https://github.com/tachsin/photonoxide/pull/14))
- add the full-vector finite-difference mode solver, with shift-and-invert Arnoldi ([#16](https://github.com/tachsin/photonoxide/pull/16))
- add mirror walls to the vector mode solver, and validate its corners against Hadley ([#20](https://github.com/tachsin/photonoxide/pull/20))
- add group index, dispersion, loss and mode tracking ([#21](https://github.com/tachsin/photonoxide/pull/21))
- add exact multilayer slab modes and leaky waves by transfer matrices ([#22](https://github.com/tachsin/photonoxide/pull/22))
- add a PML to the vector mode solver for leaky modes ([#23](https://github.com/tachsin/photonoxide/pull/23))
- add the effective index method, with its error against the vector solver ([#24](https://github.com/tachsin/photonoxide/pull/24))
- add bends: an exact bent slab, and bent cross-sections in the vector solver ([#26](https://github.com/tachsin/photonoxide/pull/26))
- add a vector mode's full fields, power and coupling into another mode ([#28](https://github.com/tachsin/photonoxide/pull/28))
- add Marcatili's approximation, validated against the vector solver in its regime ([#29](https://github.com/tachsin/photonoxide/pull/29))
- add planar profiles by 1D finite differences, with a PML ([#30](https://github.com/tachsin/photonoxide/pull/30))
- add the studio's mode viewer, with sweeps over wavelength and width ([#31](https://github.com/tachsin/photonoxide/pull/31))

### <!-- 4 -->Documentation

- add examples, each reproducing a published result ([#18](https://github.com/tachsin/photonoxide/pull/18))
- add the banner, logo and README badges ([#19](https://github.com/tachsin/photonoxide/pull/19))
- add the project site, method write-ups and example outputs ([#25](https://github.com/tachsin/photonoxide/pull/25))
- describe 0.2 as released ([#35](https://github.com/tachsin/photonoxide/pull/35))

## [0.1.1](https://github.com/tachsin/photonoxide/compare/v0.1.0...v0.1.1) - 2026-10-01

### <!-- 0 -->Added

- check the material data against Li, Malitson and Luke, and validate silica against Malitson's Table I ([#11](https://github.com/tachsin/photonoxide/pull/11))

### <!-- 4 -->Documentation

- cite the book's page and words for the 220 nm on 2 um SOI stack ([#13](https://github.com/tachsin/photonoxide/pull/13))

## [0.1.0](https://github.com/tachsin/photonoxide/releases/tag/v0.1.0) - 2026-10-01

0.1 Foundations (ROADMAP.md).

### <!-- 0 -->Added

- add the error type, and CI on Linux, macOS and Windows ([#1](https://github.com/tachsin/photonoxide/pull/1))
- add typed lengths, wavelengths and frequencies, and the e^(-iwt) convention ([#2](https://github.com/tachsin/photonoxide/pull/2))
- add materials with their provenance: Si (Li 1980), SiO2 (Malitson 1965), Si3N4 (Luke 2015) ([#3](https://github.com/tachsin/photonoxide/pull/3))
- add planar shapes, layer stacks with SOI and nitride presets, and structures ([#4](https://github.com/tachsin/photonoxide/pull/4))
- add job files, run directories, event records with exact replay, and stops ([#5](https://github.com/tachsin/photonoxide/pull/5))
- add the validation harness, its report and the photonoxide validate command ([#6](https://github.com/tachsin/photonoxide/pull/6))
- add the studio window, structure jobs and permittivity rasters ([#7](https://github.com/tachsin/photonoxide/pull/7))
- read refractiveindex.info files: its nine formulas, tabulated n, k and nk, with their provenance ([#8](https://github.com/tachsin/photonoxide/pull/8))

### <!-- 3 -->Changed

- raise the minimum Rust to 1.95 for eframe and egui 0.36 ([#9](https://github.com/tachsin/photonoxide/pull/9))
