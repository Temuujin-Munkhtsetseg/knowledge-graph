## [0.6.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.5.0...v0.6.0) (2025-07-17)

### :sparkles: Features

* **ci:** build windows binaries ([a22116c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a22116c24dc8e99ec64f77c88bb7f4ef36229e71)) by Bohdan Parkhomchuk
* **ci:** release binaries ([0551f8a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/0551f8a9469b19fc7735ec1dba4562417cd07a5c)) by Bohdan Parkhomchuk
* **db:** added atomicity for schema creation, indexing, and reindexing kuzu operations ([eca52a1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/eca52a1324cec36c15997d154ef36513e1b2795c)) by Michael Usachenko
* **db:** adding server side repository processing with c bindings ([db3146d](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/db3146dee88affa9f57a24b3596a53f8f75081dc)) by Omar Qunsul
* **db:** decoupling kuzu query generation from execution to support strict transactionality ([faa2cac](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/faa2cac8d0e00f2c5154ca78b6cbf9e449dad251)) by Michael Usachenko
* **reindexing:** listen for new workspaces in watcher, and remove closed workspaces ([9fd1f1e](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/9fd1f1ea210b7f4790e8bde27fca70638ac0f682)) by Michael Usachenko
* **reindexing:** realtime reindexing MVP integration ([95ed7c4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/95ed7c4fe636133ac80fb273af57bfd75a22761f)) by Michael Usachenko
* **reindexing:** realtime reindexing MVP pt 1 - basic handling for reindexing events and jobs ([0e54868](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/0e54868fd252b065dd37bc90094ad5237c2135b4)) by Michael Usachenko
* **reindexing:** reindexing MVP Part 2 - adding reindexing methods to the IndexingExecutor ([7d311b7](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7d311b7e1072d8b4a3d98f9957143fe73dcd0762)) by Michael Usachenko

### :bug: Fixes

* **ci:** fix windows builds ([28c8adf](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/28c8adfd6ec6fbbd99ba9c4e84e2e98a7323283b)) by Bohdan Parkhomchuk
* **db:** indexing regression where parquet relationship file check is too strict ([5a42dd1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5a42dd1f11c702a606072363d01ec9a4c64ceb90)) by Michael Usachenko
* **db:** not dropping db on workspace delete causes PK conflicts in future indexing runs ([558113a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/558113a7a9bb3afa27c223c5cafa0a2d26bf9df3)) by Michael Usachenko

### :repeat: Chore

* **ci:** do not depend on frontend assets ([ef51947](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ef5194744280e14e83d68332d1df7279905e4be6)) by Bohdan Parkhomchuk
* **ci:** use medium mac runners ([250a986](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/250a98675e1e4bd3b129835771fc4af51e602208)) by Bohdan Parkhomchuk
* **db:** bump kuzu version to latest ([e938b36](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e938b36e3eb042a2d9d3a692ec36111f610283af)) by Michael Usachenko

## [0.5.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.4.0...v0.5.0) (2025-07-08)

### :sparkles: Features

* **axum:** graceful shutdown handling for http server ([fae3870](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/fae38709f2125fc1effb1c948837c5b2ff87e056)) by Michael Usachenko
* **ci:** use nextest for tests and report generation ([ad386e8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ad386e8218ff37163c7200409689b0cc1d2f1ca2)) by Bohdan Parkhomchuk
* **db:** enforcing short lived connections for all interactions with kuzu ([f32e3a8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/f32e3a88df46c88704b64cb8bb02bd7c5c4c2171)) by Michael Usachenko
* **devex:** add common tasks to mise ([2887021](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/288702120f3d9ffa9f1f24c2c24bc83cfadb67ca)) by michaelangeloio
* **http-server, panel:** add explorer to panel ([ea53699](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ea53699ab4f1d37248c2ebf80c49e52dac2f498d)) by Michael Angelo Rivera
* **http-server:** add initial graph endpoint ([30dddc0](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/30dddc0f250bb2a0a070f116560fcd83f7b3d155)) by michaelangeloio
* **http-server:** add neighbors endpoint ([6e75eb1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6e75eb163fc27a4eafd51527dd82608204f095fc)) by michaelangeloio
* **http-server:** delete workspace endpoint ([81ae26b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/81ae26b616dc1bdbd3f88c097f19609ca1b06006)) by michaelangeloio
* **http-server:** fifo-queue ([19a5d3e](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/19a5d3eb99371f3b666dcaf66bb6e703501a92e2)) by Michael Angelo Rivera
* **mcp:** add simple list projects tool ([0c0da1a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/0c0da1a294ba37dcdac3c3b9c37caacfd075750b)) by Bohdan Parkhomchuk
* **mcp:** implement first set of tools for mcp ([e4a1bd7](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e4a1bd7344d14fc64d238ff8f64591e04e38c4ce)) by Bohdan Parkhomchuk
* **mcp:** serve MCP over HTTP and SSE ([0b22921](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/0b2292119f7b1b962c3918616b0060d9bd25c2ce)) by Jean-Gabriel Doyon
* **panel, http-server:** add node search support ([c3737d3](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c3737d354e05a93e9ae76ed0bf0cf6a88a541b54)) by michaelangeloio
* **panel:** add node click functionality ([eb9eb44](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/eb9eb44d87857b055ad25e0b4c22e07d43a1b219)) by michaelangeloio
* **panel:** introduce knowledge graph panel ([1bdd323](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1bdd32340e8ce27ea089ce1ffafa72ac30762889)) by Michael Angelo Rivera
* **querying:** create result mappers for query library ([d3036b1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/d3036b192bfeca778ac2ae9d128d43ecfda6b389)) by Jean-Gabriel Doyon

### :bug: Fixes

* **cli:** register workspace folder ([161ac11](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/161ac11e20a0a0f0650781e8f28f161d9c0c8328)) by michaelangeloio
* **http-server:** event bus types ([8468309](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/846830932cf52d14b7314623d776a2348afa573c)) by michaelangeloio
* **http:** save gkg-http-server lock file in the home directory ([dfd287e](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/dfd287ec57882076fa565d113c4fe3e91b0a80ef)) by Jean-Gabriel Doyon
* **mcp:** fix invalid MCP type ([3eb3d52](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/3eb3d5200363e76935603750d6c6610b79c6cfa5)) by Jean-Gabriel Doyon

### :zap: Refactor

* **database:** remove workspace manager ([4c0e0ab](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/4c0e0ab4a6ec859afe9d509b88eb5bd78da0fab3)) by michaelangeloio
* **http-server:** refine response structs ([8efda87](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8efda8756f532d701d3d155a17a0bedf8bbdc624)) by michaelangeloio
* **indexer:** make indexer depend and use database crate ([ab8dddc](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ab8dddc8607806e6947fad2e7987b4b18d9e7090)) by Jean-Gabriel Doyon
* **querying:** move querying in database crate ([1f7c0fd](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1f7c0fd752931830e7998ca30a7bea6a5fa25d38)) by Jean-Gabriel Doyon

### :repeat: Chore

* **deps:** add shadcn deps ([d573c6a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/d573c6a2bbcd36fcfa52135180317fabe4d9b3a5)) by michaelangeloio
* **deps:** remove .eslintrc.json ([8182a9c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8182a9c56f9b6f1ec9a4b3cf89baa6631e01304b)) by michaelangeloio
* **deps:** update eslint and prettier ([7179fe2](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7179fe2c5e8454e00e95a2505279097937dd6d0a)) by michaelangeloio
* **deps:** update gitalisk to version v0.3.1 ([35dc2d1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/35dc2d1dfab8e0c471a28efc70179af1abcb50a2)) by michaelangeloio
* **deps:** upgrade deps and rmcp ([df71a79](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/df71a79ecc596cf7d0b606c3b486885631fdae92)) by michaelangeloio
* **devex:** remove build.rs script ([212364c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/212364cc4aa49f4bb152366bab561ca99ebb6401)) by michaelangeloio
* **hooks:** make pre-commit hook auto-fix ([45c7a1f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/45c7a1fc047f0e5c541685495cd78bfd5a439e36)) by Jean-Gabriel Doyon
* **tests:** only expose database testing package for test modules ([1fcbfb2](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1fcbfb2ff91b37e3f4f952d415b20ecf2e342d23)) by Jean-Gabriel Doyon

## [0.4.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.3.0...v0.4.0) (2025-07-03)

### :sparkles: Features

* **ci:** use prebuilt docker images ([22913bf](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/22913bfad6e2b1bca2fa2bba5da45a5a7c422fd5)) by Bohdan Parkhomchuk
* **http-server:** events endpoint ([877f03a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/877f03aa69e1074c91837b4dd644f7220c672c19)) by Michael Angelo Rivera
* **http-server:** serve assets ([c526260](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c52626096b4cc855159d04fc32a4f15ccb9dcf02)) by Michael Angelo Rivera
* **http-server:** workspace list endpoint ([ec9e730](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ec9e73097bcd066738910f0da8a065122fe962dc)) by Michael Angelo Rivera
* **indexer, http-server, cli:** introduce event bus ([4af98fc](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/4af98fcd0cc35c39f34ac7c2450e8161ca418783)) by Michael Angelo Rivera
* **indexer:** incremental re-indexing for repositories mvp ([041e3c1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/041e3c1a3a3df472fe8ee1ec177e327be69473d1)) by Michael Usachenko
* **mcp:** layout extensible tools architecture ([95b8b58](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/95b8b5844d328c7d276dae9c66ce7a4afb6b187d)) by Jean-Gabriel Doyon
* **querying:** create multi-purpose querying service ([0a085aa](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/0a085aab69576ddb412a01c73218c1a59bd941e7)) by Jean-Gabriel Doyon

### :repeat: Chore

* **deps:** bump gitalisk to 0.3.0, gitlab-code-parser to 0.5.0 ([4003cca](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/4003ccaeee52f4eae8c3e85725aa80751a58e606)) by Michael Usachenko

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
