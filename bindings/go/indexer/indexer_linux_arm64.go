//go:build linux && arm64
// +build linux,arm64

package indexer

/*
#cgo LDFLAGS: -l:libindexer_c_bindings.a -ldl -lm -lstdc++ -lpthread
*/
import "C"
