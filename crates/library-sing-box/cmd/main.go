package main

import (
	"errors"
	"fmt"
	core "github.com/lingting-projects/rust-nc"
	"os"
)

func main() {
	if len(os.Args) < 1 {
		fmt.Printf("unkonow command")
		os.Exit(1)
	}

	cmd := os.Args[1]
	switch cmd {
	case "start":
		handleStart()
	case "json2srs":
		handleJsonToSrs()
	default:
		fmt.Printf("未知命令: %s\n", cmd)
		os.Exit(1)
	}
}

func handleStart() {
	configPath := os.Args[2]
	workDir := os.Args[3]

	err := core.Start(configPath, workDir)
	if !errors.Is(core.Nil, err) {
		fmt.Printf("启动失败: %s\n", err.Error())
		os.Exit(err.ToInt())
	}
}

func handleJsonToSrs() {
	jsonPath := os.Args[2]
	srsPath := os.Args[3]
	err := core.JsonToSrs(jsonPath, srsPath)
	if !errors.Is(core.Nil, err) {
		fmt.Printf("转换失败: %s\n", err.Error())
		os.Exit(err.ToInt())
	}
	fmt.Printf("成功将 %s 转换为 %s\n", jsonPath, srsPath)
}
