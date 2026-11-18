# Contributing

Contributions to this evaluation scaffold are welcome, including research notes, documentation, and changes to the Rust crate setup. By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md). For security concerns, follow [SECURITY.md](SECURITY.md) and do not report them publicly.

## Report an issue

Open an issue with the relevant commit, Rust version, operating system, expected behavior, actual behavior, and steps to reproduce. Do not describe unimplemented language behavior as a defect in a working interpreter.

## Development setup

Install Rust 1.93 or later. From the repository root, run:

```sh
cargo fmt --all -- --check
cargo check --locked
```

## Pull requests

- Target `develop` for ongoing development; `main` is reserved for changes prepared for release.
- Keep each pull request focused and describe the change and how to review it.
- List the verification commands you ran.
- Preserve copyright and license notices. Check license compatibility for third-party material and disclose added dependencies with their purpose and license.
- Cite reliable sources when proposing claims about SL and identify unresolved source limitations.

## License for contributions

Contributions are distributed under the GNU General Public License version 3 or, at your option, any later version. See [LICENSE](LICENSE).
