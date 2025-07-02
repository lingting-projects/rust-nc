use log::error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use web_view::*;

// 加载状态枚举
#[derive(Clone, Copy, PartialEq)]
enum LoadingState {
    InitSystem,
    InitDb,
    CheckUpdate,
    LoadingAssets,
    Completed,
}

impl LoadingState {
    fn progress(&self) -> u8 {
        match self {
            LoadingState::InitSystem => 0,
            LoadingState::InitDb => 25,
            LoadingState::CheckUpdate => 50,
            LoadingState::LoadingAssets => 75,
            LoadingState::Completed => 100,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            LoadingState::InitSystem => "正在初始化系统",
            LoadingState::InitDb => "正在初始化数据库",
            LoadingState::CheckUpdate => "正在检查更新",
            LoadingState::LoadingAssets => "正在加载资源",
            LoadingState::Completed => "系统初始化完成",
        }
    }

    fn message(&self) -> &'static str {
        match self {
            LoadingState::InitSystem => "准备系统环境",
            LoadingState::InitDb => "正在初始化数据库",
            LoadingState::CheckUpdate => "正在检查更新",
            LoadingState::LoadingAssets => "正在加载资源",
            LoadingState::Completed => "正在进入系统",
        }
    }

    // 获取带前缀的窗口标题
    fn window_title(&self) -> String {
        format!("nc-{}", self.title())
    }
}

fn main() {
    // 保存临时值到变量，延长生命周期
    let initial_title = LoadingState::InitSystem.window_title();

    // 创建WebView，使用带前缀的标题
    let mut webview = web_view::builder()
        .title(&initial_title) // 使用变量引用
        .content(Content::Html(include_str!("loading.html")))
        .size(800, 600)
        .resizable(true)
        .debug(true)
        .user_data(())
        .invoke_handler(|webview, arg| {
            // 处理JavaScript调用
            println!("Received message from JS: {}", arg);
            Ok(())
        })
        .build()
        .unwrap();

    // 在单独的线程中模拟加载过程
    let webview_handle = webview.handle();
    thread::spawn(move || {
        let states = vec![
            LoadingState::InitSystem,
            LoadingState::InitDb,
            LoadingState::CheckUpdate,
            LoadingState::LoadingAssets,
            LoadingState::Completed,
        ];

        for state in states {
            error!("Loading: {}", state.title());
            println!("Loading: {}", state.title());

            // 更新窗口标题和加载进度
            let result = webview_handle.dispatch(move |wv| {
                // 构建JavaScript代码
                let js_code = format!(
                    "window.to({}, '{}', '{}', '{}');",
                    state.progress(),
                    state.title(),
                    state.message(),
                    "info"
                );

                // 设置带前缀的窗口标题
                let title = state.window_title();
                wv.set_title(&title)?;
                // 执行JavaScript更新UI
                wv.eval(&js_code)
            });

            // 处理可能的错误
            if let Err(e) = result {
                error!("Failed to update UI: {}", e);
                break;
            }

            // 模拟耗时操作
            thread::sleep(Duration::from_secs(2));
        }
    });

    // 运行WebView事件循环
    if let Err(e) = webview.run() {
        error!("WebView error: {}", e);
    }
}