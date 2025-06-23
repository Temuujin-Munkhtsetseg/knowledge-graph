# Release process

Knowledge Graph is released to multiple destinations, as there will be multiple build targets.

### Ad-hoc releases

Ad-hoc releases are permitted:

- If you are a maintainer, follow the steps in [Perform release](#perform-release) below.
- Not a maintainer? Request an ad-hoc release in the `#f_knowledge_graph` Slack channel, or contact the Release DRI.

## Perform release

Perform the following steps to release a new version of the extension.

1. In `#f_knowledge_graph`, announce the release is about to be published.
1. Open a [main branch pipeline](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/pipelines?page=1&scope=all&ref=main)
   on a commit you want to publish as a release. This commit must be after any previous release commits.
1. Locate the `publish-release::manual` job and start it.
   - The version update, tagging, and creating the GitLab release all happen automatically.
   - After the version bump is done, a release tag is pushed to the repository.
1. Open the newly created [tag pipeline](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/pipelines?scope=tags&page=1) and confirm that all jobs succeeded.
1. In `#f_knowledge_graph`, announce the publishing was successful.

## Release automation with Semantic Release

We use [semantic-release](https://github.com/semantic-release/semantic-release) plugin to automate the release process.

Semantic release offers plugins that allow us to automate various steps of the release process.

| Plugin                                                                                                     | Description |
|------------------------------------------------------------------------------------------------------------|-------------|
| [`@semantic-release/commit-analyzer`](https://github.com/semantic-release/commit-analyzer)                 | Analyzes commits since the last release to identify which version bump to apply (patch, minor or major). |
| [`@semantic-release/release-notes-generator`](https://github.com/semantic-release/release-notes-generator) | Generates release notes based on the commit messages. |
| [`@semantic-release/npm`](https://github.com/semantic-release/npm)                                         | Writes the npm version. Can also be used to publish the package to an npm registry, but we rely on our own script. |
| [`@semantic-release/git`](https://github.com/semantic-release/git)                                         | Commits file changes made during the release and pushes them to the repository. |
| [`@semantic-release/gitlab`](https://github.com/semantic-release/gitlab)                                   | Creates a GitLab release and a Git tag associated with it. Uploads related release artifacts, such as the extension file, and generates comments to issues resolved by this release. |
