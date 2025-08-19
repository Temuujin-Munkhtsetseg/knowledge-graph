---
title: Go Bindings
description: A reference for the Go Bindings that are used for Server Side Indexing
---

Knowledge Graph is also used on the server side by [Zoekt Indexer](https://gitlab.com/gitlab-org/gitlab-zoekt-indexer), to parse repositories
code and generate Graph Database that is stored on the Zoekt nodes.

The interface is defined in the `indexer-c-bindings` crate.

## Static Bindings Library

As part of the release process is built also a set of
`libindexer_c_bindings.a` static bindings libraries for supported architectures, these
libraries are available as downloadable assets of each
[release](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/releases). The
reason why libraries are not included directly in the repository, but needs to
be downloaded from release assets is the size of these libraries - total size
of libraries is >500 MB so we were hitting maximum `go` module size.

## Go Bindings

`bindings/go` contains a Go module which can be used to call Knowledge Graph
indexing from Go apps. Because this module depends on the pre-compiled bindings
library, this library needs to be fetched when compiling the Go app.
`go:generate` helper directive is used to make this download easier - it uses
`libindexer-download` command which is part of the go module and it takes care of
downloading proper version of the library from release assets.

Example of calling indexer from a Go app:

```go
import "gitlab.com/gitlab-org/rust/knowledge-graph/bindings/go/indexer"

+// downloads pre-compiled static bindings lib into "libindexer/" directory
+//go:generate go run gitlab.com/gitlab-org/rust/knowledge-graph/bindings/go/cmd/libindexer-download libindexer

func main() {
    repoDir := "/tmp/gitlab"
    dbDir := "/tmp/kuzu_db"
	indexer.FullIndex(repoDir, dbDir, "/tmp/parquet", 1)
}
```

Then you can compile this application with following commands:

```
# call go generate which downloads pre-compiled library:
go generate ./...

# build Go app with passing `lib/` directory to CGo:
CGO_LDFLAGS="-L$(pwd)/libindexer/lib" CGO_CFLAGS="-I$(pwd)/libindexer/include" go build
```
