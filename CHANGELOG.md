# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-05-02

### Changed
- Switched `reqwest` to `rustls-tls` only (disabled default features) to remove the `openssl-sys` dependency and unblock cross-compilation to `aarch64-unknown-linux-gnu`.

## [0.1.0] - 2026-05-02

### Added
- Initial public release.
- Dynamic discovery of Odoo models via the JSON/2 API.
- Tools for full CRUD: `list_models`, `describe_model`, `search`, `read`, `create`, `write`, `delete`, `call_method`.
- Glob-based model filtering (`MODEL_INCLUDE` / `MODEL_EXCLUDE`).
- Read-only mode (`READ_ONLY=true`) that blocks all write operations and unsafe `call_method` invocations.
- Configurable pagination (`PAGE_SIZE`) with `has_more` / `next_offset` metadata.
- Structured error mapping for HTTP 401 / 403 / 404 / 422 / 500 responses.

[Unreleased]: https://github.com/fgribreau/mcp-odoo/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/fgribreau/mcp-odoo/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/fgribreau/mcp-odoo/releases/tag/v0.1.0
