//go:build linux && arm64
// +build linux,arm64

package indexer

/*
#cgo LDFLAGS: -L../../../lib/linux_arm64 -l:libindexer_c_bindings.a -ldl -lm -lstdc++ -lpthread
*/
import "C"
