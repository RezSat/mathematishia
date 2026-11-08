# Python release procedure

The Python bindings live in `crates/python` and are built with maturin and PyO3. The repository has wheel-install tests and a source-distribution rebuild check. A release workflow has not been configured.

## First release checklist

1. Reserve or verify the PyPI project name and confirm the publishing account has the required access.
2. Select a version and tag. Align Rust and Python package metadata; document compatibility and supported platforms.
3. Run the full Rust checks, the installed wheel matrix and the source-distribution rebuild. Test published examples against built artifacts.
4. Configure a protected GitHub environment and a PyPI Trusted Publisher bound to the release workflow and environment.
5. Add a release-only workflow with `contents: read` and narrowly scoped `id-token: write` permissions. Use `pypa/gh-action-pypi-publish` with artifacts produced by the approved build.
6. Optionally validate the workflow with TestPyPI, which requires its own publisher configuration.
7. Publish after release approval. Install packages from PyPI into fresh environments and test imports, error classifications, polynomial operations and exact algebraic-root records. Archive artifact hashes.

No publishing job should run on ordinary pushes to `main`.

References:
- https://docs.pypi.org/trusted-publishers/
- https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/
- https://www.maturin.rs/distribution.html
