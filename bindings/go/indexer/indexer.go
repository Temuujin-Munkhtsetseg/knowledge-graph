package indexer

/*
#include <stdlib.h>
#include "c_bindings.h"
*/
import "C"
import (
	"unsafe"
)

func FullIndex(repoPath, dbPath, parquetPath string, threadNum uint16) uint16 {
	cRepoPath := C.CString(repoPath)
	defer C.free(unsafe.Pointer(cRepoPath))
	cDbPath := C.CString(dbPath)
	defer C.free(unsafe.Pointer(cDbPath))
	cParquetPath := C.CString(parquetPath)
	defer C.free(unsafe.Pointer(cParquetPath))
	cThreadNum := C.ushort(threadNum)

	cResult := C.execute_repository_full_indexing(cRepoPath, cDbPath, cParquetPath, cThreadNum)

	return uint16(cResult)
}
