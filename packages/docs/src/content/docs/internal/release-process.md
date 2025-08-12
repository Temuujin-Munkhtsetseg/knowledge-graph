---
title: Release Process
description: Documentation for the gkg release process.
---

# Release process

gkg binaries and static C bindings are released as a generic package under the repository's [package registry](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/packages).

### Ad-hoc releases

Ad-hoc releases are permitted:

- If you are a maintainer, follow the steps in [Perform a release](#perform-a-release).
- If you are not a maintainer, request an ad-hoc release in the `#f_knowledge_graph` Slack channel or contact the release DRI.

## Perform a release

Perform the following steps to release a new version of `gkg`.

1. In `#f_knowledge_graph`, announce that the release is about to be published.
1. Open a [main branch pipeline](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/pipelines?page=1&scope=all&ref=main)
   on a commit you want to publish as a release. This commit must be after any previous release commits.
1. Locate the `start-release` job and start it.
   - The version update, tagging, and the creation of the GitLab release all happen automatically.
   - After the version bump is done, a release tag is pushed to the repository.
1. Open the newly created [tag pipeline](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/pipelines?scope=tags&page=1) and confirm that all jobs succeeded.
1. Open the [releases](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/releases) page and confirm that all
   binary files and their checksums are uploaded under the new release tag.
1. Confirm that `#f_knowledge_graph` has the release notes published.

## Release automation with semantic-release

We use the [semantic-release](https://github.com/semantic-release/semantic-release) plugin to automate the release process.

`semantic-release` offers plugins that allow us to automate various steps of the release process.

| Plugin                                                                                                     | Description                                                                                                                                                                                                                                                    |
| ---------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`@semantic-release/commit-analyzer`](https://github.com/semantic-release/commit-analyzer)                 | Analyzes commits since the last release to identify which version bump to apply (patch, minor, or major).                                                                                                                                                      |
| [`@semantic-release/release-notes-generator`](https://github.com/semantic-release/release-notes-generator) | Generates release notes based on the commit messages.                                                                                                                                                                                                          |
| [`@semantic-release/npm`](https://github.com/semantic-release/npm)                                         | Writes the npm version. It can also be used to publish the package to an npm registry, but we rely on our own script.                                                                                                                                          |
| [`@semantic-release/exec`](https://github.com/semantic-release/exec)                                       | Executes the `./scripts/semantic-release-prepare.sh` script, which updates the versions of all npm packages in the workspace to the new version. It also writes the new version to the `.VERSION` file to allow the `cargo-update` job to update cargo crates. |
| [`@semantic-release/git`](https://github.com/semantic-release/git)                                         | Commits file changes made during the release and pushes them to the repository.                                                                                                                                                                                |
| [`@semantic-release/gitlab`](https://github.com/semantic-release/gitlab)                                   | Creates a GitLab release and a Git tag associated with it. Uploads related release artifacts, such as executables files, static bindings, and their hash sums. Relies on `GL_TOKEN` to be set in CI.                                                           |
