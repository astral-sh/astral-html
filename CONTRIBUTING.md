# Contributing

## Releases

Releases can only be performed by Astral team members.

Before the first release, configure the protected `release-gate` and `release`
environments and release tag rules through
[Astral's GitHub policies](https://github.com/astral-sh/github-policies), and enable
**Allow GitHub Actions to create and approve pull requests** in the repository's
Actions settings. Register `astral-html` in
[crates-policies](https://github.com/astral-sh/crates-policies).
Run its **Apply** workflow to bootstrap the crate and configure Trusted Publishing
for this repository's `release.yml` workflow and `release` environment.

1. Run the **Prepare release** workflow from `main` with the exact Cargo version,
   without a leading `v`.
2. Review and merge the generated version-bump pull request.
3. Run the **Release** workflow from `main` with the same version.
4. Approve the protected `release-gate` deployment.

The release workflow verifies that the requested version matches `Cargo.toml`,
performs a Cargo publish dry run, and publishes through crates.io Trusted
Publishing. After publication succeeds, it creates the matching `v<version>`
tag and GitHub release. The publish step is the protected `release` deployment
and is safe to retry if the crate version already exists on crates.io.
