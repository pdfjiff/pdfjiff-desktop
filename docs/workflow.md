# Development workflow

The exhaustive product repository is the development source. This public repository has its own Git history and accepts desktop contributions.

1. Develop and test the desktop application in the private source tree.
2. Export only the reviewed desktop source into a clean public worktree. Public Markdown is maintained here rather than copied from private docs.
3. Run frontend build, Rust format, test, Clippy, secret scanning, and the private-material audit.
4. Commit with the public GitHub identity and push this repository only.
5. Review public contributions here, then import their commits or patches into the private source tree and rerun both private and public checks.

Never merge the private repository history into this repository. Never resolve synchronization by copying the entire private tree. Update the exact `pdfjiff-core` revision deliberately after its public CI succeeds.

