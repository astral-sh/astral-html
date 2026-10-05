# Contributing

## Releases

Releases can only be performed by Astral team members.

### Setup

Before the first release, configure the protected `release-gate` and `release`
environments and release tag rules through
[Astral's GitHub policies](https://github.com/astral-sh/github-policies).
Register `astral-html` in
[crates-policies](https://github.com/astral-sh/crates-policies) and run its
**Apply** workflow to bootstrap the crate and configure Trusted Publishing for
this repository's `release.yml` workflow and `release` environment. Crates.io
authentication uses OIDC; this repository does not need a `CARGO_REGISTRY_TOKEN`
secret.

Give the existing `astral-automations-bot` and `astral-releases-bot` GitHub Apps
access to this repository. Create an `automations` environment restricted to
`main` and configure these secrets:

| Environment   | Secret                 | Value                                                     |
| ------------- | ---------------------- | --------------------------------------------------------- |
| `automations` | `STS_API_URL`          | Astral's automation-broker base URL, without `/exchange`. |
| `automations` | `OPENAI_API_KEY`       | An API key for the Codex changelog rewrite.               |
| `release`     | `RELEASES_STS_API_URL` | Astral's release-broker base URL, without `/exchange`.    |

The brokers read `.github/secure-token-service.json` and
`.github/secure-token-service-release.json` from `main`, so merge these policies
before running the workflows.

### Prepare and publish

1. Run **Prepare release** from `main`. Leave `version` empty for automatic
   detection, or provide an exact stable Cargo version without a leading `v`.
   The first release defaults to the version already in `Cargo.toml`.
2. Review the generated version changes and changelog, then merge the release
   PR.
3. Run **Release** from `main` with the prepared version. Select **Dry-run** to
   validate the package and release notes without publishing.
4. When publishing, approve the protected `release-gate` deployment.

Preparation uses Rooster to classify merged pull requests and determine the next
version. The `breaking` label selects a minor bump; other changes select a patch
bump. Internal changes are excluded. The workflow updates the workspace version
and both Cargo lockfiles, then uses Codex to editorialize the newest changelog
section and opens a release PR assigned to the person who started the workflow.

The release workflow verifies that the requested version matches `Cargo.toml`,
checks the prepared changelog section, performs a Cargo publish dry run, and
publishes through crates.io Trusted Publishing. After publication succeeds, it
creates the matching `v<version>` tag and GitHub release using the prepared
notes. The publish step is the protected `release` deployment and is safe to
retry if the crate version already exists on crates.io.
