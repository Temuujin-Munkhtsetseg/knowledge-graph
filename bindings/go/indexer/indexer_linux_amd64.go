//go:build linux && amd64
// +build linux,amd64

package indexer

/*
#cgo LDFLAGS: -L../../../lib/linux_amd64 -l:libindexer_c_bindings.a -ldl -lm -lstdc++ -lpthread
*/
import "C"
