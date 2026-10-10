# Contributing

Thank you for improving PDFJiff Desktop. Open an issue before a large change so the scope and public/private boundary are clear.

## Local checks

```bash
npm ci
npm run typecheck
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

Keep processing in Rust and keep the frontend command surface narrow and typed. Do not add remote content, telemetry, website code, hostname checks, credentials, generated build output, or proprietary editor code. Contributions are licensed under the same `MIT OR Apache-2.0` terms as the repository.

