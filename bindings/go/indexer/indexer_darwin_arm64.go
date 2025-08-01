//go:build darwin && arm64
// +build darwin,arm64

package indexer

/*
#cgo LDFLAGS: -lindexer_c_bindings -ldl -lm -lstdc++ -lpthread
*/
import "C"
