package main

/*
#cgo CFLAGS: -I.
#include <stdlib.h>
*/
import "C"
import (
	"context"
	"fmt"
	"github.com/sagernet/sing-box/common/srs"
	"io"
	"os"
	"os/signal"
	"syscall"

	"github.com/sagernet/sing-box"
	"github.com/sagernet/sing-box/option"
	"github.com/sagernet/sing/common/json"
)

var instance *box.Box
var ctx context.Context
var cancel context.CancelFunc

//export SingBoxStart
func SingBoxStart(configPathPtr *C.char) C.int {
	configPath := C.GoString(configPathPtr)
	// 读取配置文件
	configContent, err := os.ReadFile(configPath)
	if err != nil {
		fmt.Printf("读取配置文件失败: %v\n", err)
		return -1
	}

	background := context.Background()
	// 解析配置文件
	var options option.Options
	options, err = json.UnmarshalExtendedContext[option.Options](ctx, configContent)
	if err != nil {
		fmt.Printf("解析配置文件失败: %v\n", err)
		return -1
	}

	// 创建sing-box实例
	ctx, cancel = context.WithCancel(background)
	instance, err = box.New(box.Options{
		Context: ctx,
		Options: options,
	})
	if err != nil {
		cancel()
		fmt.Printf("创建服务失败: %v\n", err)
		return -1
	}

	// 启动sing-box
	err = instance.Start()
	if err != nil {
		cancel()
		fmt.Printf("启动服务失败: %v\n", err)
		return -1
	}

	// 监听系统信号以优雅关闭
	go func() {
		signals := make(chan os.Signal, 1)
		signal.Notify(signals, os.Interrupt, syscall.SIGTERM)
		<-signals

		// 关闭sing-box
		err = instance.Close()
		if err != nil {
			fmt.Printf("关闭服务失败: %v\n", err)
		}
		cancel()
	}()

	return 0
}

//export SingBoxRefresh
func SingBoxRefresh(configPathPtr *C.char) C.int {
	if ctx == nil || instance == nil {
		fmt.Printf("未启动")
		return 0
	}
	configPath := C.GoString(configPathPtr)
	// 读取新配置文件
	configContent, err := os.ReadFile(configPath)
	if err != nil {
		fmt.Printf("读取配置文件失败: %v\n", err)
		return -1
	}

	// 解析新配置
	var options option.Options
	options, err = json.UnmarshalExtendedContext[option.Options](ctx, configContent)
	if err != nil {
		fmt.Printf("解析配置文件失败: %v\n", err)
		return -1
	}

	// 关闭当前实例
	err = instance.Close()
	if err != nil {
		fmt.Printf("关闭服务失败: %v\n", err)
		return -1
	}

	// 创建并启动新实例
	ctx, cancel = context.WithCancel(context.Background())
	instance, err = box.New(box.Options{
		Context: ctx,
		Options: options,
	})
	if err != nil {
		cancel()
		fmt.Printf("创建服务失败: %v\n", err)
		return -1
	}

	err = instance.Start()
	if err != nil {
		cancel()
		fmt.Printf("启动服务失败: %v\n", err)
		return -1
	}

	return 0
}

//export SingBoxStop
func SingBoxStop() C.int {
	if instance != nil {
		err := instance.Close()
		if err != nil {
			fmt.Printf("关闭服务失败: %v\n", err)
			return -1
		}
		instance = nil
		cancel()
	}
	return 0
}

//export SingBoxJsonToSrs
func SingBoxJsonToSrs(jsonPathPtr *C.char, srsPathPtr *C.char) C.int {
	jsonPath := C.GoString(jsonPathPtr)
	srsPath := C.GoString(srsPathPtr)

	var (
		err    error
		reader io.Reader
	)

	reader, err = os.Open(jsonPath)
	if err != nil {
		fmt.Printf("打开json文件失败: %v\n", err)
		return -1
	}
	content, err := io.ReadAll(reader)
	if err != nil {
		fmt.Printf("读取json文件失败: %v\n", err)
		return -1
	}
	ruleSet, err := json.UnmarshalExtended[option.PlainRuleSetCompat](content)
	if err != nil {
		fmt.Printf("json规则读取异常: %v\n", err)
		return -3
	}

	srsFile, err := os.Create(srsPath)
	if err != nil {
		fmt.Printf("创建srs文件异常: %v\n", err)
		return -2
	}
	defer srsFile.Close()

	err = srs.Write(srsFile, ruleSet.Options, ruleSet.Version)
	if err != nil {
		srsFile.Close()
		os.Remove(srsPath)
		fmt.Printf("写入srs文件内容异常: %v\n", err)
		return -2
	}
	srsFile.Close()
	return 0
}

func main() {
	// 保持程序运行，避免DLL加载后立即退出
	select {}
}
