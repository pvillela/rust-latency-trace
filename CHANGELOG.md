# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2024-12-20

This version is fully backwards compatible with the previous version.

### Added

- Support for `LatencyTrace` to be used as a `tracing_subscriber::Layer` in addition to its use as a `tracing::Subscriber`.

### Changed

- Doc comments

## [0.5.2] - 2024-07-11

### Changed

- Fixed dev_support/Cargo.toml to work correctly with downloaded code.
- Refined use of black_box in benchmark code.
- Minor change to Cargo.toml dependencies.

## [0.5.1] - 2024-07-08

### Added

- Missing src/bench_support.rs.

## [0.5.0] - 2024-07-08

Initial release. Starts at v0.5.0 to reflect 8 months of development and almost 200 commits.
