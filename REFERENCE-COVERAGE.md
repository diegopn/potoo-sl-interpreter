# SL reference coverage

This crate follows the language described in the Universidad Nacional de Asunción SL book and the official standard-function reference. It does not emulate the SLE editor or compiler UI. Platform-specific terminal, file, keyboard, and operating-system services are exposed through the host API.

| Area in the reference | Implemented profile |
| --- | --- |
| Identifiers and reserved words (book, pp. 15–16) | Case-sensitive identifiers of at most 32 characters; ASCII letters, digits, `_`, and `ñ`/`Ñ` are accepted, with a letter or `_` first. Accented letters and reserved words are rejected as identifiers; language keywords are matched in lowercase. |
| Basic types, constants, and declarations (book, pp. 25–28; official portal) | Numeric, string, and logical values; global and subroutine-local scalar constants; predefined `TRUE`, `FALSE`, `SI`, and `NO`; typed initializers and inferred primitive initializers; default initialization for declarations without an initializer. |
| Arrays | Fixed and open vectors and n-dimensional matrices, one-based indexing, dynamic sizing with dim, nested arrays, arrays of records, structured literals, and the optional fill marker for fixed-size arrays. Open dimensions can be resized on variable and record-field targets. |
| Records and named types | Inline records, nested record fields, record aliases, aliases of other types, structural type compatibility by field order and type, positional aggregate initialization, and record assignment. Field names are unique within a record. |
| Expressions and control | Arithmetic, relational and logical operators, string concatenation, si, eval / caso, mientras, repetir / hasta, and desde. |
| Subprograms | Procedures and functions, local declarations and types, value and ref parameters, recursion, and scalar or aggregate return values. |
| Input and output | Scalar, array, and record reads; value display with the configured input and output separators. |
| Standard functions | The documented mathematical, string, conversion, array, parameter, random, and mutation functions listed in the README. `ord`/`ascii` use the reference's 0–255 character table; `str` accepts all four documented format forms; `set_stdout` validates `wt`/`at`. Host-backed functions require an embedding host. |

Known gaps:

- Terminal, file, keyboard, and operating-system functions depend on host services; the default CLI only implements the termination operation.
- Some terminal APIs and behavior vary among SLE releases. Their host contract remains based on the selected historical reference.
- The source does not define `ord` behavior for characters outside its 0–255 table; this profile reports a runtime error for them.
- Named constants are limited to primitive values and scalar constant expressions. The parser rejects direct writes and writes through reference parameters, mutating standard functions, and host output parameters during compilation. Inferred declarations are limited to the primitive types named by the official portal; typed declarations can use structured initializers.

## Sources

- [SL book, Universidad Nacional de Asunción](https://www.cnc.una.py/sl/libro-sl.pdf), especially chapters 5, 8, 9, and 11.
- [Official SL standard-function reference](https://www.cnc.una.py/sl/SL-stdf.html).
- [Official SL portal](https://www.cnc.una.py/sl/SL-index.html).
