# PDFJiff Desktop

Fast, private PDF tools for macOS, Windows, and Linux. PDFJiff Desktop processes files on your computer: no account, upload, browser, or local server is required.

> Early development release. The interface is functional, but signed installers are not published yet.

## Available now

- **Inspect** page geometry, encryption, detected signatures, and document structures.
- **Compress** with Quality, Balanced, Small, Lossless, or a target file size.
- **Merge** PDFs in an explicit order with clear warnings when signatures or unsupported structures may be lost.
- **Capabilities** reports what the packaged engine implements instead of showing unfinished tools as available.

All writes use a temporary file and publish the result only after PDF validation. Existing files require an explicit overwrite choice. Inputs and outputs are capped to reduce accidental resource exhaustion.

## Development

Requirements:

- Node.js 22.12 or newer
- Rust 1.89 or newer
- [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
npm ci
npm run typecheck
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri dev
```

Build a local application without producing an installer:

```bash
npm run tauri -- build --debug --no-bundle
```

## Architecture and scope

The React/Vite interface calls a narrow Rust command adapter inside Tauri. The adapter invokes [`pdfjiff-core`](https://github.com/pdfjiff/pdfjiff) in-process at an exact reviewed Git revision. It does not invoke a shell command or send documents over a network.

This repository contains the public local processor. PDFJiff's comprehensive editor, website application, and hostname policy are separate proprietary components and are not included or runtime-gated here. See [docs/architecture.md](docs/architecture.md) and [docs/workflow.md](docs/workflow.md).

## License

Source code is available under your choice of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE). The PDFJiff name and bird logo are not granted under those code licenses.
