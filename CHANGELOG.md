# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2022-08-10

### Changed

- Library `rrr`
  - `SchemaParseError` is now exported outside of the crate.
- CLI application `rrr`
  - To help users understand the cause of errors when parsing schemas, error messages have been improved to display error locations and diagnostic information.
  - Improved clarity of error messages when S3 bucket access fails.

## [0.3.2] - 2022-08-02

### Fixed

- Library `rrr`
  - Fix a bug in JSON escaping for strings to be escaped from the middle

## [0.3.1] - 2022-06-19

### Fixed

- CLI application `rrr`
  - Fix a bug that `-b` option does not work for "schema" subcommand.

## [0.3.0] - 2022-05-30

### Added

- CLI application `rrr`
  - New "header" subcommand to display the header in the JSON format

## [0.2.0] - 2022-05-18

### Added

- CLI application `rrr`
  - Ability to load files from Amazon S3 when an S3 URI is given as a subcommand argument.

### Fixed

- Fix build failure in Windows environments.

## [0.1.0] - 2022-04-11

### Added

- Initial preliminary release
- Library `rrr`
  - Ability to read the data
  - Ability to export the data in the JSON format
- CLI application `rrr` built on the top of the Rust library
  - 2 subcommends:
    - dump: dump the data in the specified file
    - schema: display the schema of the data in the specified file

[0.4.0]: https://github.com/noritada/rrr-rs/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/noritada/rrr-rs/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/noritada/rrr-rs/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/noritada/rrr-rs/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/noritada/rrr-rs/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/noritada/rrr-rs/releases/tag/v0.1.0
