package core

import (
	"context"
	"errors"
	"io"
	"os"
	"os/signal"
	"path/filepath"
	"runtime"
	"runtime/debug"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/sagernet/sing-box"
	"github.com/sagernet/sing-box/common/srs"
	"github.com/sagernet/sing-box/include"
	"github.com/sagernet/sing-box/log"
	"github.com/sagernet/sing-box/option"
	"github.com/sagernet/sing/common/json"
	"github.com/sagernet/sing/service/filemanager"
	"github.com/shirou/gopsutil/v4/process"
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

func isAlive(pid int) bool {
	if pid < 1 {
		return true
	}
	p, e := process.NewProcess(int32(pid))
	if e != nil || p == nil {
		log.Debug("alive: false! ", e)
		return false
	}
	name, e := p.Name()
	if e != nil || len(name) < 1 || (name != "lingting-nc" && name != "lingting-nc.exe") {
		log.Debug("alive: false! name: ", name, e)
		return false
	}
	return true
}

func setLog(color bool) {
	formatter := log.Formatter{
		BaseTime:      time.Now(),
		DisableColors: !color,
	}
	factory := log.NewDefaultFactory(context.Background(), formatter, os.Stdout, "", nil, false)
	// 工厂必须启动, 否则日志会滞留在待输出队列, 内核异常原因将无处可见
	if err := factory.Start(); err != nil {
		log.Error("内核日志初始化失败: ", err)
	}
	log.SetStdLogger(factory.Logger())
}

// 启动阶段卡住时的告警阈值。
//
// 正常启动在一秒内完成, 超过该阈值说明某个组件阻塞在内核内部;
// 此时只有协程栈能说明卡在哪一步, 因此把全部协程栈打进日志。
const stageTimeout = 20 * time.Second

// stageWatch 标记一个启动阶段的开始, 返回结束函数。
//
// 阶段超时仍未结束时打印全部协程栈, 便于定位内核内部卡住的组件;
// 该协程在阶段结束后退出, 不会泄漏。
func stageWatch(name string) func() {
	start := time.Now()
	done := make(chan struct{})
	go func() {
		select {
		case <-done:
		case <-time.After(stageTimeout):
			buf := make([]byte, 1<<20)
			buf = buf[:runtime.Stack(buf, true)]
			log.Error("启动阶段阻塞超过 ", stageTimeout, ": ", name, ", 全部协程栈:\n", string(buf))
		}
	}()
	return func() {
		close(done)
		log.Debug("启动阶段完成: ", name, ", 耗时: ", time.Since(start))
	}
}

func readConfig(ctx context.Context, path string) (option.Options, error) {
	log.Debug("读取配置文件: ", path)
	// 读取配置文件
	config, err := os.ReadFile(path)
	if err != nil {
		log.Error("读取配置文件失败: ", err)
		return option.Options{}, err
	}
	log.Debug("配置文件读取完成, 大小: ", len(config))

	options, err := json.UnmarshalExtendedContext[option.Options](ctx, config)

	if err != nil {
		log.Error("配置文件解析异常: ", err)
		return option.Options{}, err
	}
	log.Debug("配置文件解析完成, 出口数量: ", len(options.Outbounds), ", 入口数量: ", len(options.Inbounds))

	return options, nil
}

func create(configPath string, workDir string) (context.CancelFunc, *box.Box, error) {
	log.Debug("初始化内核上下文")
	// 1.14 起 box.Context 还需要证书提供者注册表，include.Context 已统一封装全部注册表
	_ctx := include.Context(context.Background())
	ctx, cancel := context.WithCancel(_ctx)

	if workDir != "" {
		log.Debug("设置工作目录: ", workDir)
		_, err := os.Stat(workDir)
		if err != nil {
			err = filemanager.MkdirAll(ctx, workDir, 0o777)
			if err != nil {
				log.Error("工作目录创建异常: ", err)
				return cancel, nil, err
			}
		}
		err = os.Chdir(workDir)
		if err != nil {
			log.Error("工作目录设置异常: ", err)
			return cancel, nil, err
		}
	}

	options, err := readConfig(ctx, configPath)
	if err != nil {
		return cancel, nil, err
	}
	// 1.14 起 Log 是指针, 配置缺少 log 字段时为 nil
	if options.Log != nil {
		options.Log.DisableColor = true
	}
	boxOptions := box.Options{
		Context: ctx,
		Options: options,
	}
	// box.New 内部会创建日志工厂、缓存文件、路由与全部入口出口,
	// 卡住时日志停在上一行, 因此用阶段看门狗在超时后打印协程栈
	done := stageWatch("box.New")
	instance, err := box.New(boxOptions)
	done()
	if err != nil {
		log.Error("内核实例创建失败: ", err)
		return cancel, instance, err
	}

	return cancel, instance, err
}

// Check 校验配置文件能否被内核正确解析与构建。
//
// 与官方 `sing-box check` 等价：完整走一遍配置解析与实例构建，随即关闭实例，
// 不真正接管流量。返回错误文本，空串表示校验通过。
func Check(configPath string, workDir string) string {
	setLog(false)
	cancel, instance, err := create(configPath, workDir)
	defer cancel()
	if err != nil {
		return err.Error()
	}
	if instance != nil {
		closeErr := instance.Close()
		if closeErr != nil {
			return closeErr.Error()
		}
	}
	return ""
}

// Start 启动内核。
//
// reload 为重载标记文件名, 由上层传入: 上层生成新配置后在工作目录创建该文件,
// 内核发现后重建实例。为空表示未启用自动重载, 此时只响应信号。
func Start(configPath string, workDir string, pid int, reload string) (SingBoxError, error) {
	setLog(false)
	osSignals := make(chan os.Signal, 1)
	signal.Notify(osSignals, os.Interrupt, syscall.SIGTERM, syscall.SIGHUP)
	defer signal.Stop(osSignals)

	// Windows 无法向子进程发送 SIGHUP, 因此重载统一走标记文件, 信号只保留给其他平台。
	reloadFile := ""
	if reload != "" {
		reloadFile = filepath.Join(workDir, reload)
		log.Info("自动重载已启用, 标记文件: ", reloadFile)
	}

	var (
		restart  atomic.Bool
		stop     atomic.Bool
		instance *box.Box
		cancel   context.CancelFunc
	)

	go func() {
		for sig := range osSignals {
			switch sig {
			case syscall.SIGHUP:
				log.Info("收到 SIGHUP 信号，标记为需要重启")
				restart.Store(true)
			default:
				log.Info("收到其他信号，标记为需要退出! ", sig)
				stop.Store(true)
			}
		}
	}()

	for {
		debug.FreeOSMemory()
		log.Info("开始创建内核实例, 配置文件: ", configPath, ", 工作目录: ", workDir)
		_c, _i, _e := create(configPath, workDir)

		if _e != nil {
			log.Error("实例创建异常: ", _e)
			// 创建异常, 但是不存在已启动实例
			if instance == nil {
				return StartCreateError, _e
			}
		} else {
			if instance != nil {
				cancel()
				err := instance.Close()
				if err != nil {
					log.Error("实例关闭异常: ", err)
				}
			}
			cancel = _c
			instance = _i
			done := stageWatch("instance.Start")
			err := instance.Start()
			done()
			if err != nil {
				log.Error("实例启动异常: ", err)
				return StartError, err
			}
		}

		for {
			if stop.Load() || restart.Load() {
				break
			}

			// 上层刷新配置后会创建标记文件, 发现即重建实例; 未启用时不做检查
			if reloadFile != "" {
				if _, err := os.Stat(reloadFile); err == nil {
					log.Info("发现重载标记, 标记为需要重启")
					// 先删除标记, 避免重建时再次触发
					if err = os.Remove(reloadFile); err != nil {
						log.Error("删除重载标记失败: ", err)
					}
					restart.Store(true)
					break
				}
			}

			if !isAlive(pid) {
				log.Error("父进程失活, 停止实例. ", pid)
				stop.Store(true)
				continue
			}
			time.Sleep(time.Second)
		}

		// 刷新配置. 重启实例
		if !stop.Load() {
			restart.Store(false)
			continue
		}
		// 其他的关闭实例
		cancel()
		err := instance.Close()
		if err != nil {
			log.Error("实例关闭异常: ", err)
			return StopError, err
		}
		return Nil, nil
	}
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
