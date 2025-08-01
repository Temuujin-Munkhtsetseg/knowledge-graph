//go:build linux && amd64
// +build linux,amd64

package indexer

/*
#cgo LDFLAGS: -l:libindexer_c_bindings.a -ldl -lm -lstdc++ -lpthread
*/
import "C"
