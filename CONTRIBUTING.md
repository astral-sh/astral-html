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

Rooster chooses the next version from merged pull requests: `breaking` selects a
minor bump, other changes select a patch bump, and internal changes are
excluded. Preparation updates the workspace version and both Cargo lockfiles,
rewrites the newest changelog section with Codex, and assigns the release PR to
the person who started the workflow.

Release validates the version, changelog, and package before publishing through
Trusted Publishing in the `release` environment. It then creates the
`v<version>` tag and GitHub release with the prepared notes. Retries skip the
crate upload if that version already exists on crates.io.
