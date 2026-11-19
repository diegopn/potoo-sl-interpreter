# Security Policy

## Reporting a vulnerability

Do not publish vulnerability details in issues, pull requests, discussions, or public comments. If the private vulnerability reporting form is available on this repository's Security page, use it. Otherwise, use the [Potoo contact page](https://diegopn.github.io/#contato) only to request a private reporting channel. Do not include technical details or a proof of concept in that first message.

After arranging a private channel, include the affected commit, Rust version, operating system, reproduction steps, and impact. Remove credentials, personal information, and unrelated data.

For other project concerns, see [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Current scope

This crate parses and executes SL source. It does not sandbox programs. The embedding host controls terminal, file, keyboard, operating-system, and process services; enabling those services can give a program corresponding access. The command-line runner connects only the services it implements. Do not run untrusted source with a host that grants access to files or external commands.
