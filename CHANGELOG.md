## [0.3.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.2.0...v0.3.0) (2025-06-30)

### :sparkles: Features

* finishing schema migration and e2e test suite for kuzu ([bfcfe94](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/bfcfe94d982819c1e6996dde2337ebf2420a7ae4)) by Michael Usachenko
* **http-server:** add ability to run dev server ([0f8249c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/0f8249cfaa665954caf0566c219eefd0a5db2d9b)) by michaelangeloio
* **http-server:** introduce type safe bindings for typescript ([fb43490](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/fb43490958f83a9d5ffac6f70df465114d3c9d07)) by Michael Angelo Rivera
* **http:** use port 27495 if we can ([205dbbe](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/205dbbe440cc880f52a4f418606b08f6f12b8012)) by Bohdan Parkhomchuk
* **logging:** implement structured logging with files and stdout ([b018a22](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b018a22dfa1881668d390ada8891c9ceb1f29951)) by Bohdan Parkhomchuk
* **mcp:** make starting HTTP server optionally take a mcp config path ([6d54e40](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6d54e40c51a8bfe432f573b7363aa07d97376be6)) by Jean-Gabriel Doyon

### :bug: Fixes

* **ci:** disable toolchain auto update during release ([9d6aa01](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/9d6aa0191f4c52e279507677a647f8ef1488ecbe)) by Michael Angelo Rivera
* **ci:** mr title check ([1f1e58c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1f1e58c389e27cf1fd2af172252f32855f2a9a84)) by Michael Angelo Rivera
* **http-server:** workspace index endpoint ([25576f8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/25576f809699a604657d5f7a6367d97ff262ea28)) by Michael Angelo Rivera
* rust-analyzer toolchain issue with upgrade to latest rust stable ([45bf82b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/45bf82b7556c849e90bd9e182b57ba46793b5551)) by Michael Usachenko
* **workspace-manager:** change data dir to use home dir ([14e0345](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/14e03450c9a6a5ceffa6c263d2e8c7c2738ed3b0)) by michaelangeloio

### :zap: Refactor

* **http-server, cli:** use argument dependency injection ([adcafea](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/adcafeab181acf0b57f84aac2b408b96494fb5ac)) by Michael Angelo Rivera
* **mcp:** Only update mcp file if the url changed ([7d4192c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7d4192c7f68af61af7bc0ecd324ed088dca1026d)) by Jean-Gabriel Doyon
* **mcp:** use official Rust MCP SDK types ([6c7dcd1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6c7dcd1787691ed6f51d02f3e5fc5b0ae3e85a10)) by Jean-Gabriel Doyon
* **workspace:** improve workspace status aggregation and lifecycle management ([184c633](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/184c6336edad8ab8a7081881d4342acf59623410)) by michaelangeloio

### :repeat: Chore

* **ci:** Add rust check package job ([a43f84c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a43f84c4e0144cb87000c6a3dcb0ffa22316ce3a)) by Jean-Gabriel Doyon
* **deps:** update all dependencies to latest ([fa37907](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/fa379074beb8406c5686855da4d08b153cf61ae9)) by michaelangeloio
* **git:** Add installable pre-commit hook ([5df668c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5df668c89313a4cbdd8a8cd20e1c0e5aabd3d36c)) by Jean-Gabriel Doyon
* performance regression fix regarding indexer core use ([14c4e6f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/14c4e6fdef99bb1722317a73d08b5f9fe1cf3ceb)) by Michael Usachenko

## [0.2.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.1.0...v0.2.0) (2025-06-27)

### :sparkles: Features

* **ci:** add semantic-release configuration and automation ([06c5e7c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/06c5e7c65b1b3c9a583695c16fd40471ab9808b6)) by Jean-Gabriel Doyon
* **ci:** check mr title for conventional commit ([ee39c2c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ee39c2c57f03e9033c38e991cbdef06f85474283)) by Michael Angelo Rivera
* **cli:** rename to gkg ([e5c4831](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e5c48314b8677ac2682540644b9fb1cba6e65764)) by Michael Angelo Rivera
* **db:** upgrading schema ([ced3209](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ced3209ae20ff9877a1424280c9bab7ae5d3891b)) by Michael Usachenko
* **deps:** upgrade to rustc v1.88 ([e33af66](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e33af6678a886c99175afd7b5857b429b0a5f272)) by Michael Angelo Rivera
* end-to-end indexer ([623319c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/623319c9c1e61920707563bcdc7b9bba94517edc)) by Michael Angelo Rivera
* **http:** implement http server integrated into workspace manager ([3d38e5d](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/3d38e5d05f2d3e0d4fc6ce9f808a04e345b7a8df)) by Bohdan Parkhomchuk
* **mcp:** expose MCP endpoint over HTTP ([6be7881](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6be788166bbcbc0291d6c7ec6cdbeddce03b293e)) by Jean-Gabriel Doyon
* **releases:** include chore and other types in changelog ([a8b8593](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a8b85934bf40ab470279d9bc911cff25ff761706)) by michaelangeloio
* **workspace-manager:** implement data directory ([892129e](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/892129e514f9858a040d9528c24cad9a6070d869)) by Michael Angelo Rivera
* **workspace-manager:** implement manifest ([b12bf74](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b12bf747ecbcafd44a50026f9e648409df21d32d)) by Michael Angelo Rivera
* **workspace-manager:** implement state service ([557b90c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/557b90c5871ed015054d52aa8c7672e782fd9561)) by Michael Angelo Rivera
* **workspace-manager:** implement workspace manager ([5d7f1d9](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5d7f1d9d8b2861e7458e0642d25f70f3322801a7)) by Michael Angelo Rivera
* **workspace-manager:** integrate into indexer ([a98a0f8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a98a0f807ba27c57effcad63fcacb5bedb86cd6d)) by Michael Angelo Rivera

### :bug: Fixes

* **cli:** fix logging initialization ([f96dbab](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/f96dbab71f77a52ffae86865b7fa78088c6be3fe)) by Michael Angelo Rivera
* **releases:** add @semantic-release/commit-analyzer ([8129269](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8129269e410805f6f509b95a4b30f3f0e53480bc)) by Michael Angelo Rivera
* **releases:** update check-mr-title permissions ([74a2965](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/74a2965d662cd0cb7202dbde5553983266e13684)) by michaelangeloio

### :white_check_mark: Tests

* setup unit test reporting ([b94fd86](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b94fd86d0d797b44ef13848f7f35d9243deb317a)) by michaelangeloio

### :repeat: Chore

* add default merge request template ([1341a64](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1341a64da8489d56b21170ca2fcaf9ae572972ad)) by Jean-Gabriel Doyon
* add issue template ([39bb4d0](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/39bb4d0a25468f1b6eeec46127f6b1b3026c6efa)) by michaelangeloio
* **ci:** add http server to semantic release update ([eddeef9](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/eddeef9b9927ab27958ecc7659135c06783d315f)) by Bohdan Parkhomchuk
* **ci:** adding full backtrace for rust unit tests ([ca14469](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ca144695e84f4214430fcdb6e1dc5ed24cfffee8)) by michaelangeloio
* update readme ([c526a08](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c526a08385cb3f9ad59ad0743ee944ea042dff86)) by michaelangeloio
