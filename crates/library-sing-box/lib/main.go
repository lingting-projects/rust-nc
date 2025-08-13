package main

/*
#cgo CFLAGS: -I.
#include <stdlib.h>
*/
import "C"
import core "github.com/lingting-projects/rust-nc"

//export SingBoxStart
func SingBoxStart(config_path_ptr *C.char, work_dir_ptr *C.char) C.int {
	configPath := C.GoString(config_path_ptr)
	workDir := C.GoString(work_dir_ptr)
	e := core.Start(configPath, workDir)
	return C.int(e.ToInt())
}

//export SingBoxJsonToSrs
func SingBoxJsonToSrs(json_path_ptr *C.char, srs_path_ptr *C.char) C.int {
	jsonPath := C.GoString(json_path_ptr)
	srsPath := C.GoString(srs_path_ptr)
	e := core.JsonToSrs(jsonPath, srsPath)
	return C.int(e.ToInt())
}

func main() {
	// 保持程序运行，避免加载后立即退出
	select {}
}
