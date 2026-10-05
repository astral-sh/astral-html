# Contributing

## Releases

Releases can only be performed by Astral team members.

### Setup

Before the first release, configure the protected `release-gate` and `release`
environments and release tag rules through
[Astral's GitHub policies](https://github.com/astral-sh/github-policies).
Register `astral-html` in [crates-policies](https://github.com/astral-sh/crates-policies)
and run its **Apply** workflow to bootstrap the crate and configure Trusted
Publishing for this repository's `release.yml` workflow and `release` environment.
Crates.io authentication uses OIDC; this repository does not need a
`CARGO_REGISTRY_TOKEN` secret.

Release preparation uses Astral's existing credential broker and
`astral-automations-bot` GitHub App, following Serc. Give the App access to this
repository and create an `automations` environment restricted to `main` with:

| Secret | Value |
| --- | --- |
| `STS_API_URL` | The existing Astral credential-broker base URL, without `/exchange`. |
| `OPENAI_API_KEY` | An API key for the Codex changelog rewrite. |

The broker reads `.github/secure-token-service.json` from `main`, so merge the
policy before running preparation. It grants the preparation workflow access to
create the release branch and pull request. No new broker deployment or App
private-key secret is required.

### Prepare and publish

1. Run **Prepare release** from `main`. Leave `version` empty for automatic
   detection, or provide an exact stable Cargo version without a leading `v`.
   The first release defaults to the version already in `Cargo.toml`.
2. Review the generated version changes and changelog, then merge the release PR.
3. Run **Release** from `main` with the prepared version.
4. Approve the protected `release-gate` deployment.

Preparation uses Rooster to classify merged pull requests and determine the next
version. The `breaking` label selects a minor bump; other changes select a patch
bump. Internal changes are excluded. The workflow updates the workspace version
and both Cargo lockfiles, then uses Codex to editorialize the newest changelog
section and opens a release PR assigned to the person who started the workflow.

The release workflow verifies that the requested version matches `Cargo.toml`,
checks the prepared changelog section, performs a Cargo publish dry run, and
publishes through crates.io Trusted Publishing. After publication succeeds, it
creates the matching `v<version>` tag and GitHub release using the prepared notes.
The publish step is the protected `release` deployment and is safe to retry if
the crate version already exists on crates.io.
