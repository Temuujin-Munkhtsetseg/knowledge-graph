# Go Bindings

`bindings/go` contains a Go module which can be used to call Knowledge Graph
indexing from Go apps.

As part of the release process is built also
`lib/<arch>/libindexer_c_bindings.a` static library which is then used in this
Go modules.

Example of calling indexer from Go:

```go
import "gitlab.com/gitlab-org/rust/knowledge-graph/bindings/go/indexer"

func main() {
    repoDir := "/tmp/gitlab"
    dbDir := "/tmp/kuzu_db"
	indexer.FullIndex(repoDir, dbDir, "/tmp/parquet", 1)
}
```
