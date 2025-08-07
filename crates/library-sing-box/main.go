package main

/*
#cgo CFLAGS: -I.
#include <stdlib.h>
*/
import "C"
import (
	"context"
	"io"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/sagernet/sing-box"
	"github.com/sagernet/sing-box/common/srs"
	"github.com/sagernet/sing-box/include"
	"github.com/sagernet/sing-box/log"
	"github.com/sagernet/sing-box/option"
	"github.com/sagernet/sing/common/json"
	"github.com/sagernet/sing/service/filemanager"
)

var (
	instance    *box.Box
	ctx         context.Context
	cancel      context.CancelFunc
	initialized bool
)

type Code = C.int

const (
	StartCreateError Code = -iota - 1
	StartError
	StartAlready
	StopError
	FileCreateError
	FileOpenError
	FileReadError
	FileWriteError
	RuleReadError
)

func setLog(color bool) {
	formatter := log.Formatter{
		BaseTime:      time.Now(),
		DisableColors: !color,
	}
	factory := log.NewDefaultFactory(context.Background(), formatter, os.Stdout, "", nil, false)
	logger := factory.Logger()
	log.SetStdLogger(logger)
}

func readConfig(ctx context.Context, _path *C.char) (option.Options, error) {
	path := C.GoString(_path)
	log.Debug("读取配置文件: ", path)
	// 读取配置文件
	config, err := os.ReadFile(path)
	if err != nil {
		log.Error("读取配置文件失败: ", err)
		return option.Options{}, err
	}

	options, err := json.UnmarshalExtendedContext[option.Options](ctx, config)

	if err != nil {
		log.Error("配置文件解析异常: ", err)
		return option.Options{}, err
	}

	return options, nil
}

func create(configPathPtr *C.char, workDirPtr *C.char) (*box.Box, error) {
	c := box.Context(context.Background(), include.InboundRegistry(), include.OutboundRegistry(), include.EndpointRegistry())
	ctx, cancel = context.WithCancel(c)

	workDir := C.GoString(workDirPtr)

	if workDir != "" {
		log.Debug("设置工作目录: ", workDir)
		_, err := os.Stat(workDir)
		if err != nil {
			filemanager.MkdirAll(ctx, workDir, 0o777)
		}
		err = os.Chdir(workDir)
		if err != nil {
			log.Error("工作目录设置异常: ", err)
			return nil, err
		}
	}

	options, err := readConfig(ctx, configPathPtr)
	if err != nil {
		return nil, err
	}
	options.Log.DisableColor = true
	options.Log.Level = "trace"
	boxOptions := box.Options{
		Context: ctx,
		Options: options,
	}
	instance, err := box.New(boxOptions)

	return instance, err
}

func isRunning() bool {
	if !initialized {
		initialized = true
		setLog(false)
	}
	return instance != nil
}

//export SingBoxRunning
func SingBoxRunning() C.int {
	if !isRunning() {
		return 0
	}
	return 1
}

//export SingBoxStart
func SingBoxStart(configPathPtr *C.char, workDirPtr *C.char) C.int {
	if isRunning() {
		log.Warn("服务已启动")
		return StartAlready
	}
	var (
		err error
	)
	instance, err = create(
		configPathPtr,
		workDirPtr,
	)
	if err != nil {
		cancel()
		log.Error("创建服务失败: ", err)
		return StartCreateError
	}

	// 启动sing-box
	err = instance.Start()
	if err != nil {
		cancel()
		instance = nil
		log.Error("启动服务失败: ", err)
		return StartError
	}

	// 监听系统信号以优雅关闭
	go func() {
		signals := make(chan os.Signal, 1)
		signal.Notify(signals, os.Interrupt, syscall.SIGTERM)
		<-signals

		// 关闭sing-box
		err = instance.Close()
		if err != nil {
			log.Error("关闭服务失败: ", err)
		}
		cancel()
	}()

	return 0
}

//export SingBoxStop
func SingBoxStop() C.int {
	if !isRunning() {
		return 0
	}

	err := instance.Close()
	if err != nil {
		log.Error("关闭服务失败: ", err)
		return StopError
	}
	instance = nil
	cancel()
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
		log.Error("打开json文件失败: ", err)
		return FileOpenError
	}
	content, err := io.ReadAll(reader)
	if err != nil {
		log.Error("读取json文件失败: ", err)
		return FileReadError
	}
	ruleSet, err := json.UnmarshalExtended[option.PlainRuleSetCompat](content)
	if err != nil {
		log.Error("json规则读取异常: ", err)
		return RuleReadError
	}

	srsFile, err := os.Create(srsPath)
	if err != nil {
		log.Error("创建srs文件异常: ", err)
		return FileCreateError
	}
	defer srsFile.Close()

	err = srs.Write(srsFile, ruleSet.Options, ruleSet.Version)
	if err != nil {
		srsFile.Close()
		os.Remove(srsPath)
		log.Error("写入srs文件内容异常: ", err)
		return FileWriteError
	}
	srsFile.Close()
	return 0
}

func main() {
	// 保持程序运行，避免DLL加载后立即退出
	select {}
}
