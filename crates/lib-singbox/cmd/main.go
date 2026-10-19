package main

import (
	"errors"
	"fmt"
	core "github.com/lingting-projects/rust-nc"
	"github.com/sagernet/sing-box/log"
	"os"
	"strconv"
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
	case "check":
		handleCheck()
	case "json2srs":
		handleJsonToSrs()
	default:
		fmt.Printf("未知命令: %s\n", cmd)
		os.Exit(1)
	}
}

func handleStart() {
	pid := 0

	configPath := os.Args[2]
	workDir := os.Args[3]
	argPid := os.Args[4]
	if len(os.Args) >= 5 && argPid != "" {
		pidInt, err := strconv.Atoi(argPid)
		if err != nil {
			log.Warn("传入的Pid值异常! ", argPid)
		} else {
			pid = pidInt
		}
	}

	code, err := core.Start(configPath, workDir, pid)
	if !errors.Is(core.Nil, code) {
		// 同时输出原始错误, 便于定位内核自身的异常原因
		fmt.Printf("启动失败[%d]: %s: %v\n", code.ToInt(), code.Error(), err)
		os.Exit(code.ToInt())
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

func handleCheck() {
	configPath := os.Args[2]
	workDir := os.Args[3]
	reason := core.Check(configPath, workDir)
	if reason != "" {
		fmt.Printf("配置校验失败: %s\n", reason)
		os.Exit(1)
	}
	fmt.Printf("配置校验通过: %s\n", configPath)
}
