# Architecture

```text
React/Vite interface
        │ typed Tauri invoke calls
        ▼
Rust desktop adapter
        │ validated requests and local file access
        ▼
version-pinned pdfjiff-core
```

The WebView owns presentation only. Rust revalidates every request, reads user-selected files, runs the PDF engine on blocking worker threads, validates results, and publishes outputs atomically. No Node.js runtime ships in the application.

The initial capability manifest permits core window behavior plus native open/save dialogs. Frontend assets are bundled, CSP blocks remote content, and the application has no telemetry or application-level network feature.

The public boundary is this entire repository plus the separately versioned public core dependency. PDFJiff's website, hostname rules, and comprehensive editor are outside that boundary.

