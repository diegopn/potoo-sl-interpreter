<p align="center">
  <img src="https://diegopn.github.io/potoo-site/src/assets/images/language-interpreter-icon.svg?v=1c651ca7" alt="Potoo IDE language interpreter icon" width="120" />
</p>

<h1 align="center">SL Interpreter</h1>

<p align="center">A Rust interpreter for the documented core of SL/SLE, the Spanish educational algorithm language.</p>

<p align="center">
  <a href="https://github.com/diegopn/potoo-sl-interpreter/releases">Download</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/sl/">Website</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/sl/">User manual</a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"></a>
  <a href="https://doc.rust-lang.org/cargo/"><img alt="Cargo" src="https://img.shields.io/badge/Cargo-DEA584?style=for-the-badge&logo=rust&logoColor=white"></a>
  <img alt="Rust library" src="https://img.shields.io/badge/Rust_library-4A86CF?style=for-the-badge">
  <img alt="CLI" src="https://img.shields.io/badge/CLI-6A4C93?style=for-the-badge">
</p>

This Rust package is part of the Potoo IDE ecosystem and is distributed independently from the desktop interface. Integration with the IDE has not been implemented or verified.

## Implemented subset

- `programa`, declarations before `inicio`, and the `inicio` / `fin` program body.
- Global and subroutine-local `const` declarations with scalar constant expressions; the predefined Boolean constants are `TRUE`, `FALSE`, `SI`, and `NO`. Writes to constants are rejected during parsing, including indirect writes through reference parameters and mutating functions.
- Typed variable declarations with initializers and inferred primitive declarations such as `count = 0` or `rows = alen(matrix)`; initializer expressions run in source order before the containing body.
- Case-sensitive identifiers of up to 32 characters. They start with an unaccented ASCII letter or `_`, may contain ASCII letters, digits, `_`, `ñ`, or `Ñ`, and cannot be reserved words. Language keywords must be lowercase.
- Numeric, string, and Boolean scalar values; fixed and open vectors and nested arrays with one-based indexing.
- Inline and aliased record types, including nested records and arrays of records; dot-field access, structural assignment, structured record literals, the array-literal fill marker, record input/output, and dynamic sizing of open arrays in variables and record fields.
- Arithmetic, comparison, and Boolean expressions, plus string concatenation with `+`.
- `leer`, `imprimir`, `si` / `sino`, `eval` / `caso`, `mientras`, `repetir` / `hasta`, `desde` / `hasta` / `paso`, and `dim`.
- Procedures and functions with local declarations, recursion, scalar and aggregate return values, and live `ref` parameters that alias variables, fields, and indexed elements during the call.
- Functions may return scalar values, arrays, or records.
- Mathematical functions `abs`, `arctan`, `cos`, `exp`, `int`, `log`, `sin`, `sqrt`, and `tan`.
- String functions `ascii`, `lower`, `ord`, `pos`, `strdup`, `strlen`, `substr`, and `upper`, plus numeric conversion/formatting with `str` and `val`.
- `alen`, `ifval`, `inc`, `dec`, `intercambiar`/`swap`, `max`, `min`, `random`, `pcount`, `paramval`, and `sec`.

Array bounds, invalid types, undefined names, division by zero, and execution-limit failures return runtime errors. Input and output are provided by the embedding host; the CLI connects them to standard input and output. The `Host::environment` hook covers release-dependent terminal and file services, key input, `runcmd`, and reference outputs from screen queries. `set_ifs` and `set_ofs` are interpreted directly, including array input and display formatting. `ord` uses the reference's byte-table range and rejects characters outside it; `set_stdout` accepts only the documented `wt` and `at` modes.

## Current limits

`beep`, `cls`, `eof`, `get_*`, `readkey`, `set_stdin`, `set_stdout`, and `runcmd` require host services; the default CLI host only supports `terminar`. Inferred declarations are limited to primitive values, as specified by the official SL portal. The official site distinguishes historical language details from behavior supported by individual SLE releases, so exact compatibility for those runtime-specific areas remains edition-dependent.

See [reference coverage](REFERENCE-COVERAGE.md) for the source-by-source feature matrix.

## Library use

Call `parse(source)` and then `Interpreter::new(host).run(&program)`. Implement `Host` to provide input, output, positional parameters, and the `environment` operations needed by a program. `HostResponse` can return a value and values written through output parameters. The returned `Execution` contains the final variables and the number of executed statements.

## Command line

```sh
cargo run --locked -- source.sl
```

The CLI reads a UTF-8 source file, passes trailing command-line arguments to `pcount`/`paramval`, and uses standard input/output for `leer` and `imprimir`.

## Support

If this project is useful to you, you can support its continued development:

<p align="left">
  <a href="https://github.com/sponsors/diegopn"><img alt="Become a sponsor" src="https://img.shields.io/badge/Become_a_sponsor-EA4AAA?style=for-the-badge&logo=githubsponsors&logoColor=white"></a>
</p>

## References

- [Official SL portal, Facultad Politécnica, Universidad Nacional de Asunción](https://www.cnc.una.py/sl/SL-index.html)
- [Official SL standard functions reference](https://www.cnc.una.py/sl/SL-stdf.html)
- [Potoo SL manual](https://diegopn.github.io/potoo-site/manual/sl/)

## Project policies

- [Code of conduct](CODE_OF_CONDUCT.md)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [GPL-3.0-or-later license](LICENSE)
