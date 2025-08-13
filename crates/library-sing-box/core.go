package core

import (
	"context"
	"errors"
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

type SingBoxError int8

func (e SingBoxError) Error() string {
	switch {
	case errors.Is(e, Nil):
		return ""
	case errors.Is(e, StartCreateError):
		return "启动创建失败"
	case errors.Is(e, StartError):
		return "启动失败"
	case errors.Is(e, StartAlready):
		return "已启动"
	case errors.Is(e, StopError):
		return "停止失败"
	case errors.Is(e, FileCreateError):
		return "文件创建失败"
	case errors.Is(e, FileOpenError):
		return "文件打开失败"
	case errors.Is(e, FileReadError):
		return "文件读取失败"
	case errors.Is(e, FileWriteError):
		return "文件写入失败"
	case errors.Is(e, RuleReadError):
		return "规则读取失败"
	default:
		return "未知错误"
	}
}

func (e SingBoxError) ToInt() int {
	return int(e)
}

const (
	Nil SingBoxError = -iota
	StartCreateError
	StartError
	StartAlready
	StopError
	FileCreateError
	FileOpenError
	FileReadError
	FileWriteError
	RuleReadError
)

var (
	instance *box.Box
	ctx      context.Context
	cancel   context.CancelFunc
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

func readConfig(ctx context.Context, path string) (option.Options, error) {
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

func create(configPath string, workDir string) (*box.Box, error) {
	_ctx := box.Context(context.Background(), include.InboundRegistry(), include.OutboundRegistry(), include.EndpointRegistry())
	ctx, cancel = context.WithCancel(_ctx)

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

	options, err := readConfig(ctx, configPath)
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

func Start(configPath string, workDir string) SingBoxError {
	setLog(false)
	var (
		err error
	)
	instance, err = create(
		configPath,
		workDir,
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

	return Nil
}

func JsonToSrs(jsonPath string, srsPath string) SingBoxError {
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
	return Nil
}
