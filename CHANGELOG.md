## [0.15.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.14.0...v0.15.0) (2025-09-18)

### :sparkles: Features

* **cli:** allow start server with debug logs ([7eb6054](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7eb605451088b4f8b392c772a65793c9582abf80)) by Jean-Gabriel Doyon
* **mcp:** allow mcp tools asynchronous execution ([e9b22f6](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e9b22f63a29b15ea73c25215f0a90c8fdfee9039)) by Bohdan Parkhomchuk
* switch to cloud hsm mac signing ([3fde5d1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/3fde5d115e9fb182e0fc34d1fd949eb9aac1174c)) by Bohdan Parkhomchuk

### :bug: Fixes

* **java:** check null enclosing scope ([1e90795](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1e90795bab62963b077debddc03959293354bb81)) by Jean-Gabriel Doyon
* **java:** resolve reference to nested classes ([43ff927](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/43ff927948a6afb9ec8fcf8871383a7e21c1174a)) by Jean-Gabriel Doyon
* **mcp:** add duo MCP server as SSE ([e2eb8d0](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e2eb8d036b9df0da490cc748b92ab16009ae2a93)) by Jean-Gabriel Doyon

### :memo: Documentation

* add api reference to sidebar ([7ae9ae6](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7ae9ae613a5613d6ab306202f5f707bbd30d5e8d)) by Dennis Meister
* add documentation for allowing raw db access ([98f8e1b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/98f8e1b4792106aabd43affa368a6c1c437c5c75)) by Isaac Dawson

### :repeat: Chore

* **deps:** bump gitlab-code-parser to 0.18.1 ([68f47ac](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/68f47ac3bb7c7f48e011b51b6cf01d2cd741689e)) by Jean-Gabriel Doyon
* rename paths to absolute paths for tools ([85aec8b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/85aec8b55cc9bce0da113ec78814f7b6e0a666fb)) by Bohdan Parkhomchuk
* update wording to clarify project status ([55c2231](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/55c2231dc865ddadb9a78deeb02ecb5be84ecb94)) by Lucas Charles
* use https for semantic release isntead of ssh ([309002b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/309002b83a405b463650e16ba4f9b510a73e92f3)) by Bohdan Parkhomchuk

## [0.14.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.13.0...v0.14.0) (2025-09-16)

### :sparkles: Features

* add basic HTTP server for server-side ([eabab41](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/eabab4134860388b95fb5fdfa4037a5b2a68f31c)) by Jan Provaznik
* **indexer:** added cross-file reference resolution for Python ([8de2aa3](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8de2aa3ea0efedaf1c788f5fbdef80fabb29e43e)) by Jonathan Shobrook

### :bug: Fixes

* code sign Windows binary on default branch ([0d119c4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/0d119c415f945e8e50f0722a24d71c75541f136f)) by Stan Hu

## [0.13.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.12.0...v0.13.0) (2025-09-15)

### :sparkles: Features

* add get_definition tool ([088d974](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/088d9743d396e1eee33ffe2c6c2d1ada9e25de37)) by Bohdan Parkhomchuk
* add utils for chunked file reads ([156aa32](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/156aa321f0b7d37fcfe33217da35801c0525b26a)) by Bohdan Parkhomchuk
* **axum:** added basic health check ([593d044](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/593d044030a102c15f937834455670ef46a7c026)) by Michael Usachenko
* **cli:** allow debug builds of the gkg cli to query kuzu directly for easy experimentation ([ed9d3b9](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ed9d3b97c6e179a384fa6d23d711df934c3cfca6)) by Michael Usachenko
* **db:** remove import nodes duplication ([f7a751c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/f7a751c45935d2160117d9d4da357d265d317e8c)) by Jean-Gabriel Doyon
* **evals:** local evaluation framework for gkg part 1 - deps and config ([150656f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/150656fb702d4bf09452aac029435aa3dc756eed)) by Michael Usachenko
* **evals:** local evaluation framework for gkg part 2 - fixture download and gkg indexing ([872a1a4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/872a1a4ba08085515e89b0618883ad31711aa601)) by Michael Usachenko
* **evals:** local evaluation framework for gkg part 3 - code agent ([ad055c7](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ad055c7b074699b82c2e52707f554f3927e6b094)) by Michael Usachenko
* **evals:** local evaluation framework for gkg part 4 - swe-bench integration and report gen ([bd7c8b8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/bd7c8b871cf08f867c60f233aaf09893eed90271)) by Michael Usachenko
* **evals:** local evaluation framework for gkg part 5 - pipeline sessions ([8e613f5](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8e613f515d7b5cf317c9e9abce80c7965e5b3a20)) by Michael Usachenko
* **indexer:** add call location to imported symbol call edge ([a29f490](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a29f4906686c1d207a4eb956dbd850895ff304d3)) by Jean-Gabriel Doyon
* **indexer:** added imported symbol relationships ([bf846a1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/bf846a1e02929d553f564b72e511112a43181878)) by Jonathan Shobrook
* **indexer:** capture line ranges in call edges ([c757c6c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c757c6c76059c48eefdaad9f9e35e559332a84be)) by Michael Angelo Rivera
* **java:** create call edge between a definition and an dependency import ([505b002](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/505b0026d5cef0627c1c46bde5ca3bdcf6565233)) by Jean-Gabriel Doyon
* **java:** create reference links to imports ([6ab1894](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6ab1894b37739e12a915291c1e37eaa9b1c23bda)) by Jean-Gabriel Doyon
* **mcp:** add file context to search codebase tool ([27a2888](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/27a28887debddbf833f8e8bf04eed5b3e5f306c4)) by Jean-Gabriel Doyon
* **mcp:** add import usage tool ([287020f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/287020f308730972b337d4ae28dde89413facb59)) by michaelangeloio
* **mcp:** add repo map tool ([01cdc5f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/01cdc5f70ac85ee6f5b2d5e268689028c7b4e3a3)) by michaelangeloio
* **mcp:** auto add approvedTools for mcp registration ([ccc5079](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ccc50791716376d2ae06e3941e320b85f704b6d9)) by michaelangeloio
* **mcp:** create MCP configuration ([1f78449](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1f78449d27aae876dc7ed06e267630617ddb05f4)) by Jean-Gabriel Doyon
* **mcp:** create read_definitions tool ([d4e1283](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/d4e1283aa3b1137dd979db8e177a4f5e3665a061)) by Jean-Gabriel Doyon
* **mcp:** display tools output as pretty XML ([db784f4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/db784f4269c9a48ce847da8f492b6d6c79a51392)) by Jean-Gabriel Doyon
* **mcp:** get references tool ([7dcb1f2](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7dcb1f2566f8e1616fc175e2236709154bfcb2e3)) by Jean-Gabriel Doyon
* **mcp:** improve indexing tool description and output ([b7447fb](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b7447fbfd6c5d733e9d4dca6ae85a104989d1ca2)) by Jean-Gabriel Doyon
* **mcp:** re-add list_projects tool and switch back project enums to absolute path ([c68d66c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c68d66cd676e71e4334edebc8f218f8aec89f074)) by Jean-Gabriel Doyon
* **mcp:** remove cdata from xml output ([63f7f45](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/63f7f455ad677e49056d009a09191ef5c493804d)) by Jean-Gabriel Doyon
* **mcp:** search_codebase_definitions no longer can return full body, added system message ([fdb6cfe](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/fdb6cfe54e56c7560b89edaf2303a0de25ecbac0)) by Jean-Gabriel Doyon
* sign Windows binaries ([618c158](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/618c158d999c642b78ea5b56cbdb1bde51fd64f3)) by Stan Hu
* **ui:** display new import relationships in the graph ([4d3cdd7](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/4d3cdd7b85d9c2f8cfc5c05586f12c4466ca992d)) by Jean-Gabriel Doyon
* **ui:** display relationship type in the graph ui tooltip ([9104d92](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/9104d92de72cb86ff23131b6589fdfe485d0c88e)) by Jean-Gabriel Doyon
* **ui:** increase displayed node limits ([bd9c5a4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/bd9c5a46a6a152dcae982053c39b3541c1b610fa)) by Jean-Gabriel Doyon

### :bug: Fixes

* **docs:** fix docs light mode title ([4e21e6c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/4e21e6c9df4a8a66b2216cd06c9d37970aacd21d)) by Jean-Gabriel Doyon
* **docs:** integrity check setting is no longer required in ide ([5b99ecb](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5b99ecb5fb384d7963a67a791fb5002bf0205210)) by Bohdan Parkhomchuk
* **java:** remove package lookup by class name ([8a59736](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8a59736c2458f914bc80bc9b7a9100f08d1353b9)) by Jean-Gabriel Doyon
* update database schema manager for Windows long path support ([8a52b47](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8a52b47acef1f5e60697c296f32a8d2c4539429e)) by Amr Zaher
* **workspace-manager:** handle path trailing separator ([3620d39](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/3620d394c97ca18e51ae8973e8cf531e821494e4)) by michaelangeloio

### :zap: Refactor

* **mcp:** declare input_schema using serde_json instead of rmpc ([d255a8f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/d255a8fbfe23995862f035d710ea0cf67beadff5)) by Jean-Gabriel Doyon

### :repeat: Chore

* **db:** bump kuzu version to 0.11.2 ([1fe3c5c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1fe3c5cfc304026f95661a4898e070f1fbbc0ea8)) by Michael Usachenko
* deprecate analyze_code_files tool ([a05f5f0](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a05f5f006e20854fa63d4523d5685d8a494dfba3)) by michaelangeloio
* **deps:** bump gitalisk to v0.6.0 ([c7a9125](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c7a9125c6273c9b629cf525c2b925c47a8143011)) by Michael Usachenko
* **deps:** bump gitlab-code-parser to 0.18.0 ([28936af](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/28936afc365531b0de12428ef83f90b12d19db10)) by Jean-Gabriel Doyon
* **mcp:** rename configuration to duo configuration ([c1f78dc](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c1f78dcab1acd075a02729a7cd2eabfda91799f0)) by Jean-Gabriel Doyon
* rename http-server to http-server-desktop ([b966437](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b966437b3adc0504baa46959a2e242a67ab7e36e)) by michaelangeloio
* revert "feat(db): remove import nodes duplication" ([b797ae7](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b797ae71d36a36de566e707f55adb3de316d7384)) by Jean-Gabriel Doyon
* revert "feat(java): create reference links to imports" ([92d1cca](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/92d1ccab3f5482302bc8961054c3912a5892d5a3)) by Jean-Gabriel Doyon

## [0.12.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.11.1...v0.12.0) (2025-08-28)

### :sparkles: Features

* **db:** less hardcoded, more declarative defining of schema + management ([48401ec](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/48401ecfbd2539f00d084d625cc40bf09dc757e4)) by Michael Usachenko
* **docs:** add documentation for Python ([5098c5d](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5098c5da5bb8cb084f703ff7df89ec65ddd72dff)) by Jonathan Shobrook
* **docs:** fufill branding request ([97a72c2](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/97a72c29bba6371a635a47631f038ca69ee3288e)) by Michael Angelo Rivera
* **indexer:** added indexing for Python references ([aeb2318](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/aeb231849f762d6703313ce73dc10f225e7e6847)) by Jonathan Shobrook
* **java:** index Java cross-file reference ([e9cf740](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e9cf740852b5585a0bc9fea5b0b389cf168942ed)) by Jean-Gabriel Doyon

### :bug: Fixes

* added the prototocol to the web interface output ([1a399bb](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1a399bb81f3e3b9abf8c2ada737e94c796224697)) by Denys Mishunov
* **docs:** resolve 404 errors on hosted page ([0078ed5](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/0078ed57d289bca44ea2fb6bc9f6780e1f61ab1e)) by Adam Mulvany
* **indexer:** fix string concatenation error in ruby reference resolver ([7d7c087](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7d7c087b8bcc64685f8e947d4d15eb16250a2511)) by Michael Usachenko
* **install:** improve install script path updates ([5e691a4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5e691a44d6bf5257502aa59a221d3b13de9a3ad9)) by Bohdan Parkhomchuk
* update download command ([c83c249](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c83c249d2f58469ba4abf3642832b8a380640f42)) by Jan Provaznik
* update go version in bindings and rename module ([7899ab8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7899ab8b210336710e88b41ccbc1e9df0cf5b069)) by Omar Qunsul

### :memo: Documentation

* **pages:** fix absolute links causing 503 errors ([ba43f6c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ba43f6c8b984d0481c61a60857f079bba7541b87)) by Adam Mulvany

### :repeat: Chore

* **indexer:** use tokio-rayon::spawn instead of tokio::task::spawn_blocking ([c402db0](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c402db0860543d1042635f48f2c0d0ac2077b198)) by Michael Usachenko
* **mr:** shorten performance analysis section in merge request ([fe0865e](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/fe0865e6104111930ce1ddccf12f156e98681151)) by Jean-Gabriel Doyon

## [0.11.1](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.11.0...v0.11.1) (2025-08-16)

### :bug: Fixes

* **cli:** make server start optional ([c3422a1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c3422a1fe05a92b735d4ef2672b83b9364f68d00)) by michaelangeloio
* **docs:** fix ruby table ([ca3ff33](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ca3ff33a9dcdd404b42c87ba2778c6c55e353997)) by Jean-Gabriel Doyon

### :memo: Documentation

* add index-cmd docs ([5cec4f0](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5cec4f0e48be6f820568166207011ba5c6123752)) by Michael Angelo Rivera
* add ruby docs ([3cea28b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/3cea28b8d24801ba556335a72a84138be4fc091f)) by michaelangeloio
* minor tweaks ([c0dc88c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c0dc88c7991e416cf54bd009a1de40c171b4060e)) by Michael Angelo Rivera

### :repeat: Chore

* **ci:** fix and readd windows release ([44b67ba](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/44b67ba4ce8313d1c7be63ff31ba909d23796f69)) by Bohdan Parkhomchuk
* **docs:** fix docs landing page links ([ad9b44b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ad9b44b9a60a612e3f4113690a95e884de24aafc)) by Jean-Gabriel Doyon

## [0.11.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.10.0...v0.11.0) (2025-08-15)

### :sparkles: Features

* **cli:** implement clean command ([40157c4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/40157c4a3ad65ef60e109dc45816590ad752344d)) by Bohdan Parkhomchuk
* **csharp:** add definitions and imports indexing ([6b46f6a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6b46f6a274f75b52f3795f387b340af06012133d)) by Bohdan Parkhomchuk
* **indexer:** add rust definitions ([7789601](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/77896016c4b0112f85293a748833a1a75af693d8)) by michaelangeloio
* **indexer:** add rust imports ([c7c7cef](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c7c7cef65e7dd624aee1fbe6ba50766094c1b0dc)) by Michael Angelo Rivera
* **kotlin:** index intra-file references ([e2730c8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e2730c8fd48576b96e18be0a013030245d13bddc)) by Jean-Gabriel Doyon
* **mcp:** add a synchronous re-index tool ([52db203](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/52db2034fe004b8bf7911a6d4e52d4637f49fb24)) by Jean-Gabriel Doyon
* **mcp:** add get symbol references tool ([65cc1a4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/65cc1a4874c251d77bc551fe4c4682a8dc849679)) by Jean-Gabriel Doyon
* **playground:** make project search work ([41a89b1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/41a89b191b3fd6d3aa229c9435bb1f96184d85e9)) by Jean-Gabriel Doyon
* **ruby:** index ruby references ([1ce3c98](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1ce3c986b21f2281baa11cd1e45bf682185cf429)) by Michael Angelo Rivera
* **ts:** integrate ts intra-file references into the indexer ([07fe67f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/07fe67fafe3e765fbc4aec39ca89aa4d6cba1ceb)) by Michael Usachenko
* **ui:** add re-index button in the playground project view ([c6db22f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c6db22fa1aa22ccaefe17df7090ae1c21d4d2ecf)) by Jean-Gabriel Doyon

### :bug: Fixes

* **ci:** add libclang-dev to build release ([26bf324](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/26bf324d9e513d04e19e5af6766b162afe77ce38)) by Michael Angelo Rivera
* update bindings include path ([84821c8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/84821c89adcce9a6d12bb7bfd8306cc7aae821fc)) by Jan Provaznik

### :memo: Documentation

* add clean command and missing reindex mcp tool ([b0ecb6f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b0ecb6f7e8ccecc7070e6b91af2b810688028be3)) by Bohdan Parkhomchuk
* add contribute section with dev setup ([8ffe08b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8ffe08b9668e0f7caee08fc406353a2d39abb91d)) by Bohdan Parkhomchuk
* clarify role of gitlab-code-parser ([2cf27f8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/2cf27f891ff0be60bd226fb73db47b60a4c1aa5b)) by michaelangeloio
* update server endpoint docs ([44ad0e0](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/44ad0e0c118b955171d346177e510b4ea3e6a052)) by Michael Angelo Rivera

### :repeat: Chore

* **deps:** bump code parser to v0.15.1 ([64c5968](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/64c596888424939d81df14abd2f98a2f0076674d)) by Michael Usachenko
* **deps:** update all deps to latest ([f9a7e64](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/f9a7e64f3cc3a9da73e8cf9a7a3d9d167816cf76)) by michaelangeloio
* **deps:** upgrade gitlab-code-parser to 0.15.0 ([9061647](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/9061647dff35ab231193a64865df45e6a800f5bb)) by michaelangeloio
* **docs:** add Java coverage documentation ([c745d1f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c745d1ff14324b4e860a75821f76f0d12cb20204)) by Jean-Gabriel Doyon
* **docs:** add Kotlin docs ([3594cd0](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/3594cd0df77cc8928bc1399b30695641886063c5)) by Jean-Gabriel Doyon
* **docs:** ts/js docs and reindexing disclaimer ([fc539ce](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/fc539ce09fa7e80fcff54cfa2980c7284a8cac31)) by Michael Usachenko
* **gkg:** rename stats output ([1e6d118](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1e6d118eb0071a12732c61562f50c5f5b9005a82)) by michaelangeloio
* **indexer:** show summary for missing definitions ([a8a1471](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a8a14716973a7fe209595e700a5108f038f11afb)) by michaelangeloio
* **mcp:** rename analyze code file MCP tool to be plural ([5c91c6c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5c91c6c515c0546744fd82f73087bee2b12f43db)) by Jean-Gabriel Doyon
* **mcp:** rename reindex tool to index tool ([019589f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/019589fcbd193b1ed16dd24edbeda8fbd9c692df)) by Bohdan Parkhomchuk
* rename kuzu database file ([dbb48e8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/dbb48e8afb143cd678f82b95ee8b338ce2f09f20)) by michaelangeloio
* switch windows release off temp ([9aa836b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/9aa836b5bd0fca42d1503d2a6eaf83dce5a44a29)) by Bohdan Parkhomchuk
* update LICENSE ([55da5c2](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/55da5c24be50957b95d1df51d77a5509fe06deb5)) by michaelangeloio

## [0.10.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.9.0...v0.10.0) (2025-08-14)

### :sparkles: Features

* **cli:** daemon mode for unix systems ([91ed1d4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/91ed1d402b2011d286818aa7ef64028756efdfad)) by Bohdan Parkhomchuk
* display import in playground ([4758f64](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/4758f6423b21f2289323577d0fe34008af84ed1a)) by Jean-Gabriel Doyon
* **gkg:** support server start and stop commands ([b7fa400](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b7fa4009c6b7f6b450392d218a4d4f927dc08f4a)) by Bohdan Parkhomchuk
* include libindexer header file in release ([16832bf](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/16832bf1eff12f2b21d8f0e9bb8a2d26c45e59c3)) by Jan Provaznik
* **java:** index Java imports ([7bb5261](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7bb526179a300c77e1ad8d8417c9ce734a14f7ae)) by Jean-Gabriel Doyon
* **kotlin:** index Kotlin imports ([07bc2f6](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/07bc2f68c5c71d2cf005a346ac9df3bb9ccfb8bf)) by Jean-Gabriel Doyon
* **mcp:** add analyze code file and search tools ([6e8d406](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6e8d406f2a83f79959a86a73096316486b5fb69a)) by Jean-Gabriel Doyon
* **mcp:** add MCP documentation ([f19329b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/f19329ba3d2d450f73acb57812c5abd2b325cc94)) by Jean-Gabriel Doyon
* **perf:** enable mimalloc ([ff13947](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ff13947e6e7d38755929523d4ff04eb488bb7afb)) by Michael Usachenko

### :bug: Fixes

* fix download path ([e2fa21e](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e2fa21e90c8c44be807b5e95d969e183a04dacf0)) by Jan Provaznik
* **gkg:** lock file race condition ([c39cd69](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c39cd695e10bd052da5c375b41f28669a0694899)) by Michael Angelo Rivera
* **indexer:** properly cleanup db file ([864dace](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/864dacecd1198b51fc2d894bf8efeac43d0b9d13)) by Michael Angelo Rivera
* make stderr logs async and lossy ([6b79ef6](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6b79ef64d0d4f863aa9bd823c01e43e9274555af)) by Bohdan Parkhomchuk
* **panel:** show total nodes in panel graph preview ([ba24e66](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ba24e6675eb80c45a10764cc30b7baa64e3228c9)) by michaelangeloio
* properly output gkg version in cli ([30c5c2a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/30c5c2a6d50e6b8f78a58be3c321a68eb7db52a9)) by Bohdan Parkhomchuk
* stop spamming stderr for server foreground logs ([18a380e](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/18a380e033013345c2c250e66ecd4427d5841bd7)) by Jean-Gabriel Doyon

### :memo: Documentation

* add bug template ([2df3259](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/2df3259f77b99944c0a8a3e270b7c096a08b16d8)) by Bohdan Parkhomchuk
* add gitlab pages link to readme ([9ac20a7](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/9ac20a73b3c8e81eaca0e5537b116dc78d3a2135)) by Bohdan Parkhomchuk
* add lang support page, ide integration, extend troubleshoot ([a287982](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a287982ab70cea831c1ed5148bb2567d44e00b19)) by Bohdan Parkhomchuk
* update server start/stop commands, describe detached mode ([e52163d](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e52163d522c26ae9dc1b4bc25b9492055238dadd)) by Bohdan Parkhomchuk

### :zap: Refactor

* **indexer:** support mandatory FQN in parser-core definitions ([a075bac](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a075baca7a436238f236faefa167e17b9af66686)) by Vitali Tatarintev
* **server:** enhance server logs and modularize tracing functions ([bf64e3f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/bf64e3f192f2d40b5f9176b0a00a687830c6bc27)) by Fern

### :fast_forward: Performance

* **indexer:** pipelined async I/O with bounded CPU parsing, async executors, consolidate file I/O ([4d17208](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/4d172080dbeca26f965fe1678ab78cc677c9420c)) by Michael Angelo Rivera

### :repeat: Chore

* **build:** require frontend assets to build ([f7a89c6](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/f7a89c6f345f1b4941c332b40e1d7a94376b283b)) by Bohdan Parkhomchuk
* **cli:** split main file ([c2d16ce](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/c2d16ce209892f189c3f1ded7764ee1c2bc3a014)) by Bohdan Parkhomchuk
* **db:** move database service to database crate ([b68fc22](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b68fc2225e349db7a04d28696e20586c82117d84)) by Michael Usachenko
* **deps:** upgrade rmcp and deps ([ddd7ad1](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ddd7ad16c83fae6e904a07153df141f5c75fd72d)) by Michael Angelo Rivera
* **deps:** upgrade rust to 1.89.0 ([328ce89](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/328ce893621e5b11d48400e3000d4f5a4a6a4024)) by michaelangeloio
* **indexer:** support ts ast-grep removal in indexer ([5a84a1f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5a84a1fbf39a1b3c347bb9ba9713a4e87fbb4a16)) by Michael Usachenko
* **logging:** replace std print lines with tracing logging ([3d67ee6](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/3d67ee654287b8626a878ab30014068e09be2dfd)) by Bohdan Parkhomchuk

## [0.9.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.8.0...v0.9.0) (2025-08-04)

### :sparkles: Features

* **ci:** properly handle package versions for releases ([84bb0f9](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/84bb0f93fe1a2a16384d851c24f93c63fd078ead)) by Bohdan Parkhomchuk
* **indexing:** added range support for definitions to node id generator ([f782cd3](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/f782cd3c89bda233fc188bc3e7e7ba909eafe626)) by Michael Usachenko
* update bindings distribution ([91b3954](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/91b395459403b1f48f839dc5352eeeb4ed53717f)) by Jan Provaznik

### :bug: Fixes

* **ci:** xtasks add or edit comment ([4846e40](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/4846e4099c3e4c4e7fb67eb58dfe4853dfcdd75b)) by michaelangeloio

### :memo: Documentation

* moving go bindings docs ([d10890f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/d10890f6ab2d179ecc22044ff487ff7ea55c598a)) by Omar Qunsul

### :repeat: Chore

* add mise fix-all command ([fefd47a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/fefd47a4d7861dffe410d17ed782439f07ce3633)) by Michael Angelo Rivera
* ignore lib/ in code ([aa27c56](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/aa27c56c5687da23d85d0946cacbabfb805b331c)) by Omar Qunsul
* **indexer:** switch match_info to range ([cb73fa7](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/cb73fa769ac2818429f0c44d4b2998291ca20931)) by michaelangeloio
* make binaries smaller ([7a346fd](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7a346fd4bf4efd3e791ddfc7a7ec5065fd40e834)) by Dmitry Gruzd
* move release process documentation ([e19634a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e19634a74ddcc05027bcf3e212bd48de2539b2a0)) by Michael Angelo Rivera
* re-adding .gitattributes because we had lfs files ([43c68c5](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/43c68c520908102926ccde509f13c09f54e9a1a2)) by Omar Qunsul
* **xtask:** remove version check ([505aa4b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/505aa4b3eede9077cea579be5a2f61147d9da403)) by michaelangeloio

## [0.8.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.7.0...v0.8.0) (2025-07-30)

### :sparkles: Features

* **bench:** use dynamic kuzu for macos bench ([5ed7592](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5ed759202045665da6b55c27b9e761075f4a2cef)) by Bohdan Parkhomchuk
* **docs:** add docs framework ([80e3ef2](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/80e3ef2a52b00a636de5c6932eedca0f4f9ef77a)) by Michael Angelo Rivera
* **reindexing:** enable re-indexing for imports ([5e91f7c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5e91f7cf4a1ad757b66071a6082dc0f13f45ab28)) by Michael Usachenko

### :bug: Fixes

* **ci:** fix releases for darwin ([39fe684](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/39fe6840365de03498d8dcb266d62e3a412f2555)) by Bohdan Parkhomchuk

### :repeat: Chore

* **ci:** enforce newlines ([3ca9b74](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/3ca9b7450138c37dedb7976d930a822f8e7349cf)) by michaelangeloio
* **ci:** fix arch variable coming from CI ([35029d6](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/35029d65b36de887f8f56e7e0c4216d3421dba21)) by Bohdan Parkhomchuk
* **deps:** bump kuzu to latest (0.11.1) ([2abf2a4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/2abf2a47291a30ddd4639d8b4946f6b0a3eadbad)) by Michael Usachenko
* **deps:** update gitlab-xtasks ([49ebd57](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/49ebd571a8a2abd64cd91f359af36739bcb9b473)) by Michael Angelo Rivera
* **deps:** update lock file ([8df939f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8df939f7d5b08bc4227937381e46eaa6b722d6f4)) by michaelangeloio
* downgrade go version ([9e2359f](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/9e2359f5ed8c9fc363bb4b05eaa5097b33063531)) by Jan Provaznik
* **mise:** add docs command to mise ([8aae243](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/8aae243596ec9b0f9f90f57973709561956ddf00)) by michaelangeloio
* update go bindings module ([fb45c4d](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/fb45c4d33ff6f3001da85b67e23adea1aafed3d8)) by Jan Provaznik

## [0.7.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.6.0...v0.7.0) (2025-07-29)

### :sparkles: Features

* add go bindings package ([ae96f42](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ae96f42eb17d528ab7c8713131ad90fd22aa45c0)) by Jan Provaznik
* **benchmark:** add GDK benchmark test ([a3db486](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a3db48696d563f01f80e595ec8d6ac0b2e4fa1fc)) by Michael Angelo Rivera
* **indexer:** add basic statistics to indexer ([cb6fcce](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/cb6fcced22738a6634087e5533d5bd5451cdc6b8)) by Michael Angelo Rivera
* **indexer:** added indexing support for Python definitions ([1479266](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1479266f1eb9bf1998c883ceae95faecc7d316e4)) by Jonathan Shobrook
* **indexer:** full migration to relationship type enum in indexer ([b686dcb](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b686dcb36862fb74e6fa75daed599dba927681ae)) by Michael Usachenko
* **indexer:** indexing for imported symbols in Python ([e115cb9](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e115cb929753a094526b5f18cbecb92f39b6a2d7)) by Jonathan Shobrook
* **install:** mac binary signing ([bb7f548](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/bb7f548a96331de9532290bb0c1d4098922ad3d8)) by Bohdan Parkhomchuk
* **install:** one-line installation scripts ([b49ae65](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b49ae657c0756cc460b5cf9478e546072563e9fb)) by Bohdan Parkhomchuk
* **java:** index Java definitions ([7b8a281](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7b8a2810d0245e180c47bbf176b34105ab8f9274)) by Jean-Gabriel Doyon
* **kotlin:** index Kotlin definitions ([9d96959](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/9d96959a72e629d72d80a7093334af924734f709)) by Jean-Gabriel Doyon
* **reindexing:** enforcing project level watching, better transaction isolation ([90b080c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/90b080c21b29d0acb14d1c719ed0b35b12b10bea)) by Michael Usachenko
* **ts:** implement TS/JS indexing for definitions and imports ([b1ece4b](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b1ece4b719bceee3a5baf1de37a217a24eb0374e)) by Michael Usachenko
* **watcher:** watchers can now operate over entire workspaces, or individual projects ([5371f00](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/5371f0062f43f9ee54191d9b103e76733de8f4db)) by Michael Usachenko

### :bug: Fixes

* **ci:** install git lfs in release step ([a0bba24](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a0bba24939ffa47859647622ceed895a816a0592)) by Bohdan Parkhomchuk
* **db:** kuzu node id assignments now use primary_file_path and fqn ([b1d805c](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/b1d805cfb153aa26410ce19195d00b104a5eba30)) by Michael Usachenko
* **deps:** update lock file ([a0602f2](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a0602f2d5e2cf6a771e8c7b153504bc1bda55a4f)) by michaelangeloio
* **frontend:** remove shadcn-vue dep, due to stylus dependency being removed from npm ([d9ebbf4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/d9ebbf4aca32af9f82374c26270a630477fcd225)) by Michael Usachenko
* **playground:** fix graph neighbours search ([7ef58fb](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7ef58fb53cbed726624a5b062ce396d3af4c7009)) by Jean-Gabriel Doyon
* **playground:** hovering an edge not showing tooltip, use name instead of FQN for the nodes ([3f45cf8](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/3f45cf8e747817f4a84975055d738b61ace7eafe)) by Jean-Gabriel Doyon

### :zap: Refactor

* **indexer:** reduce file parsing progress output ([1b02013](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/1b02013e08ccaa78c57698d2b44b97b3d38e894b)) by michaelangeloio

### :repeat: Chore

* **deps:** bump gitalisk to v0.4.0 and parser to v0.7.0 ([a8b0877](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/a8b0877a2574a01001fa6c4c9b32c15919b4f023)) by Michael Usachenko
* **deps:** remove unused deps ([7a0bf8a](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/7a0bf8ad330b2ddffac88936aecd112bcbbf1e23)) by Bohdan Parkhomchuk
* **docs:** fix readme typo ([6290bfa](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6290bfaad705952b1ea6dd2f842276a2543617b2)) by Bohdan Parkhomchuk
* **indexer:** added more location details to DefinitionNode and ImportedSymbolNode ([d2cd098](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/d2cd098695a95df11aaab3946721c6c5c1a5a548)) by Jonathan Shobrook

## [0.6.0](https://gitlab.com/gitlab-org/rust/knowledge-graph/compare/v0.5.0...v0.6.0) (2025-07-18)

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

* **ci:** add missing windows ci dependency ([6f9a078](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/6f9a07850451a78e072edfdbab11a70bc5820b3d)) by Bohdan Parkhomchuk
* **ci:** do not depend on frontend assets ([ef51947](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/ef5194744280e14e83d68332d1df7279905e4be6)) by Bohdan Parkhomchuk
* **ci:** use medium mac runners ([250a986](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/250a98675e1e4bd3b129835771fc4af51e602208)) by Bohdan Parkhomchuk
* **db:** bump kuzu version to latest ([e938b36](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/e938b36e3eb042a2d9d3a692ec36111f610283af)) by Michael Usachenko
* **release:** 0.6.0 [skip ci] ([59024c4](https://gitlab.com/gitlab-org/rust/knowledge-graph/commit/59024c4da2e85a0a8785aa224e203ea712e409fe)) by semantic-release-bot

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
