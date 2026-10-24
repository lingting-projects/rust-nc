package main

import (
	"errors"
	"flag"
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
	args := flag.NewFlagSet("start", flag.ExitOnError)
	// 短key便于手动启动测试, 长key便于脚本与日志辨认
	var (
		configPath string
		workDir    string
		pid        int
		reload     string
		debug      bool
	)
	for _, name := range []string{"c", "config"} {
		args.StringVar(&configPath, name, "", "配置文件路径")
	}
	for _, name := range []string{"w", "work"} {
		args.StringVar(&workDir, name, "", "工作目录")
	}
	for _, name := range []string{"p", "pid"} {
		args.IntVar(&pid, name, 0, "父进程 pid, 内核据此判断是否需要退出")
	}
	for _, name := range []string{"r", "reload"} {
		args.StringVar(&reload, name, "", "重载标记文件名, 不传则不启用自动重载")
	}
	// 调试模式只在 ncdebug 标签构建的内核中生效, 正式构建下该参数不可用
	if core.DebugEnabled {
		for _, name := range []string{"d", "debug"} {
			args.BoolVar(&debug, name, false, "调试模式, 父进程只要求存活")
		}
	}
	if err := args.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}
	if configPath == "" || workDir == "" {
		fmt.Printf("缺少必要参数: -c 配置文件, -w 工作目录\n")
		args.Usage()
		os.Exit(1)
	}

	code, err := core.Start(configPath, workDir, pid, reload, debug)
	if !errors.Is(core.Nil, code) {
		// 同时输出原始错误, 便于定位内核自身的异常原因
		fmt.Printf("启动失败[%d]: %s: %v\n", code.ToInt(), code.Error(), err)
		os.Exit(code.ToInt())
	}
}

func handleJsonToSrs() {
	args := flag.NewFlagSet("json2srs", flag.ExitOnError)
	var jsonPath, srsPath string
	for _, name := range []string{"c", "config"} {
		args.StringVar(&jsonPath, name, "", "json 规则集路径")
	}
	for _, name := range []string{"o", "out"} {
		args.StringVar(&srsPath, name, "", "srs 规则集输出路径")
	}
	if err := args.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}
	if jsonPath == "" || srsPath == "" {
		fmt.Printf("缺少必要参数: -c json 规则集, -o srs 输出\n")
		args.Usage()
		os.Exit(1)
	}

	err := core.JsonToSrs(jsonPath, srsPath)
	if !errors.Is(core.Nil, err) {
		fmt.Printf("转换失败: %s\n", err.Error())
		os.Exit(err.ToInt())
	}
	fmt.Printf("成功将 %s 转换为 %s\n", jsonPath, srsPath)
}

func handleCheck() {
	args := flag.NewFlagSet("check", flag.ExitOnError)
	var configPath, workDir string
	for _, name := range []string{"c", "config"} {
		args.StringVar(&configPath, name, "", "配置文件路径")
	}
	for _, name := range []string{"w", "work"} {
		args.StringVar(&workDir, name, "", "工作目录")
	}
	if err := args.Parse(os.Args[2:]); err != nil {
		os.Exit(1)
	}
	if configPath == "" {
		fmt.Printf("缺少必要参数: -c 配置文件\n")
		args.Usage()
		os.Exit(1)
	}

	reason := core.Check(configPath, workDir)
	if reason != "" {
		fmt.Printf("配置校验失败: %s\n", reason)
		os.Exit(1)
	}
	fmt.Printf("配置校验通过: %s\n", configPath)
}
