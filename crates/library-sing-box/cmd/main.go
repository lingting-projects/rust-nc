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

	err := core.Start(configPath, workDir, pid)
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
