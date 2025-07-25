//go:build darwin && arm64
// +build darwin,arm64

package indexer

/*
#cgo LDFLAGS: -L../../../lib/darwin_arm64 -lindexer_c_bindings -ldl -lm -lstdc++ -lpthread
*/
import "C"
