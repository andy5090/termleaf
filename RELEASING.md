# Releasing Termleaf

Termleaf follows Semantic Versioning:

- Patch (`0.1.1`): compatible bug fixes and small polish.
- Minor (`0.2.0`): compatible features or meaningful behavior additions.
- Major (`1.0.0`): incompatible CLI, file, configuration, or workflow changes.

## Prepare

1. Move completed entries from `CHANGELOG.md`'s `Unreleased` section into a
   new version section with the release date.
2. Update `package.version` in `Cargo.toml`; run `cargo check` to refresh
   `Cargo.lock`.
3. Update the comparison links at the bottom of `CHANGELOG.md`.
4. Run:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features
   dist plan
   ```

5. Commit and push the release preparation, then confirm the normal CI passed.

To conserve GitHub Actions usage, normal CI runs once per pull request and
cancels superseded runs. The i686 compatibility build and cargo-dist platform
matrix run only for version-tag releases; validate i686-sensitive changes
locally before tagging when possible.

## Publish

Create and push a tag that exactly matches the Cargo package version:

```bash
git tag -a v0.2.2 -m "Termleaf 0.2.2"
git push origin v0.2.2
```

The generated Release workflow builds macOS, x86_64 Linux, and i686 Linux
archives, checksums, and the common shell installer, then publishes them to
GitHub Releases. The i686 archive is supplied by the custom reusable workflow
at `.github/workflows/build-i686.yml`.

After it finishes, verify:

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/andy5090/termleaf/releases/latest/download/termleaf-installer.sh | sh
termleaf --version
```

Do not move or recreate a published version tag. If a release needs a fix,
publish a new patch version.
