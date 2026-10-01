# Changelog

All notable changes to photonoxide are documented in this file, generated from the pull request titles by [release-plz](https://release-plz.dev/).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and photonoxide adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
