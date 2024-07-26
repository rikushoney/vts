# Verilog-To-Superconducting

The Verilog-To-Superconducting (VTS) project is a framework for superconducting
field-programmable gate array (SFPGA) architecture exploration.

## Getting started

VTS is composed of many sub-projects. More information about each sub-project
is available under its respective subdirectory:

| Name            | Directory                          | Status        | Language |
| --------------- | ---------------------------------- | ------------- | -------- |
| `vts_abc`       | [crates/vts_abc](crates/vts_abc)   |               | Rust     |
| `vts_api`       | [crates/vts_api](crates/vts_api)   |               | Rust     |
| `vts_cli`       | [crates/vts_cli](crates/vts_cli)   |               | Rust     |
| `vts_core`      | [crates/vts_core](crates/vts_core) | [![CI][2]][1] | Rust     |
| Python bindings | [python/vts](python/vts)           |               | Python   |

[1]: https://github.com/rikushoney/vts/actions/workflows/vts_core_ci.yml
[2]: https://github.com/rikushoney/vts/actions/workflows/vts_core_ci.yml/badge.svg

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.
