# PyPI release checklist — not published yet

Mathematishia already has a Rust-backed Python package named mathematishia
in crates/python. It is experimental version 0.1.0 in the repository and is
built with maturin/PyO3. Building an artifact is not the same as publishing
to the public PyPI index.

## Required before the first public release

1. Confirm the PyPI project name is available, the maintainer owns the
   project, and the GitHub repository URL and package metadata are accurate.
   A pending trusted publisher does not reserve the package name.
2. Approve a public version, release notes, changelog, API compatibility
   policy and supported platform/Python matrix. Keep Cargo package versions
   and the Python project version aligned.
3. Build and install wheels for supported Linux, Windows and macOS targets;
   test the installed package and the package metadata. Build/test an sdist
   on a clean machine. Run cargo fmt/check/clippy/tests and the examples.
4. Configure a dedicated GitHub environment with release approvals. Set
   up a PyPI Trusted Publisher matching exactly the GitHub repository,
   approved release workflow filename and environment. Avoid stored,
   long-lived PyPI API tokens.
5. Create a narrowly scoped release workflow, triggered only by an
   explicitly approved release/tag. Give the publishing job only the
   id-token: write permission needed by OIDC, and contents: read.
   Use the official pypa/gh-action-pypi-publish mechanism after obtaining
   tested artifacts. Never publish on push to main.
6. Test an optional TestPyPI release and install the uploaded artifacts.
   Remember that TestPyPI and PyPI publisher configurations are separate.
7. After human approval, publish the production release, install it in
   fresh environments without source checkout, check import, exact
   polynomial operations, typed error handling and algebraic proof steps.
   Record release artifact hashes and reproducible test results.

Official references consulted November 2026:
https://docs.pypi.org/trusted-publishers/
https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/
https://docs.pypi.org/trusted-publishers/using-a-publisher/
https://www.maturin.rs/distribution.html

No PyPI account credentials are stored in this repository, and this
checklist does not configure a trusted publisher on the user's behalf.
