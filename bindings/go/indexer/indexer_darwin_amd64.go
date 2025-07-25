//go:build darwin && amd64
// +build darwin,amd64

package indexer

/*
#cgo LDFLAGS: -L../../../lib/darwin_amd64 -lindexer_c_bindings -ldl -lm -lstdc++ -lpthread
*/
import "C"
