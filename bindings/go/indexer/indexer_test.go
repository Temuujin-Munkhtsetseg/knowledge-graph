package indexer

// to run the test, build bindings library first using "mise build-lib",
// then pass the location of built library when running the test:
// CGO_LDFLAGS="-L../../../target/x86_64-unknown-linux-gnu/release/" go test

import (
	"testing"
	"path/filepath"
	"runtime"
)

func TestFullIndex(t *testing.T) {
	dir := t.TempDir()

	_, filename, _, _ := runtime.Caller(0)
	// run indexer on the current bindings directory
	repoPath := filepath.Dir(filename)

	dbPath := filepath.Join(dir, "kuzu_db")
	parquetPath := t.TempDir()

	result := FullIndex(repoPath, dbPath, parquetPath, 1)
	if result != 0 {
		t.Errorf("FullIndex returned %d, want: %d", result, 0)
	}
}
