mod assets;
mod blocks;
mod components;
mod logging;
mod main_window;
mod pages;
mod services;
mod single;
mod theme;
mod tray;
mod utils;

use gpui_kit::*;
use service_core::holder::settings::SettingsHolder;

use crate::assets::AppAssets;

fn main() {
    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        use std::ptr;
        use winapi::um::wincon::GetConsoleWindow;
        use winapi::um::winuser::{SW_HIDE, ShowWindow};
        let console_window = unsafe { GetConsoleWindow() };

        if console_window != ptr::null_mut() {
            // 隐藏窗口
            unsafe {
                ShowWindow(console_window, SW_HIDE);
            }
        }
    }
    // 日志必须在数据层之前初始化：后台同步任务的失败原因只体现在日志里。
    if let Err(error) = logging::init() {
        eprintln!("初始化日志失败: {error:#}");
    }
    // panic 必须在这里装上：界面崩溃走的是 abort，没有 hook 时原因不会进日志。
    logging::install_panic_hook();

    // 单进程锁必须在数据层与内核启动之前拿到：两个实例同时操作数据库与内核会互相破坏状态。
    let guard = match single::Guard::create() {
        Ok(guard) => guard,
        Err(error) => {
            log::error!("{error:#}");
            return;
        }
    };

    application().with_assets(AppAssets::new()).run(|cx| {
        init_quit(cx);
        init(cx);
        services::init(cx);
        // 总线必须在页面创建之前装好：页面在构造时订阅数据更新事件。
        services::init_bus(cx);
        // 内核不依赖界面：数据层就绪后立即按设置自动运行，不等窗口创建。
        if let Some(services) = services::ready(cx) {
            services::auto_run(&services);
        }
        theme::apply(cx);

        main_window::init(cx);
        // 托盘必须在主线程创建，它依赖当前线程的消息循环接收点击与菜单事件。
        // 初始化失败是致命的：常驻应用没有托盘就没有唤回窗口与退出应用的入口；
        // 返回 `false` 只表示当前平台没有托盘，界面流程照常继续。
        let tray_ready = match tray::init(cx) {
            Ok(ready) => ready,
            Err(error) => {
                log::error!("[启动] 托盘初始化失败，退出: {error:#}");
                cx.quit();
                return;
            }
        };

        // 主窗口一律以隐藏状态创建，再按启动方式决定显示还是销毁：
        // 隐藏创建不会闪过一帧空白，销毁后由托盘唤回时重建。
        let window = match main_window::open(cx, true) {
            Ok(window) => window,
            Err(error) => {
                log::error!("[启动] 创建主窗口失败: {error:#}");
                cx.quit();
                return;
            }
        };
        // 只有「托盘已创建 + 最小化启动」才销毁窗口：没有托盘就没有唤回入口，
        // 窗口必须留着，否则应用等于不见了。
        if tray_ready && SettingsHolder::software().minimize {
            log::info!("[启动] 最小化启动, 销毁主窗口, 进程留在托盘");
            main_window::close(cx, window);
        } else {
            main_window::show(cx, window);
        }

        // 锁与托盘一起存活：后续实例的唤醒信号由界面线程转成窗口唤回（窗口不在则重建）。
        single::listen(cx, guard);
    });

    // 正常退出也主动落盘：日志线程在全局守卫里，进程结束时不一定来得及写完最后一批。
    logging::flush();
}

/// 退出策略与退出路径的日志。
///
/// 托盘常驻应用不能跟着窗口走：GPUI 默认「最后一个窗口关闭即退出整个应用」，
/// 一旦窗口被系统销毁（显示配置变化、显示适配器移除等），进程会静默退出，
/// 日志里不留任何痕迹，事后无法判断退出原因。因此改成只由 [`App::quit`] 退出，
/// 并把退出路径写进日志。
fn init_quit(cx: &mut App) {
    cx.set_quit_mode(QuitMode::Explicit);

    cx.on_window_closed(|cx, window_id| {
        let remaining = cx.windows().len();
        log::info!("[退出] 窗口已关闭: {window_id:?}, 剩余窗口数量: {remaining}");
        // 窗口没了就没有东西要展示，页面轮询挂起到重新显示为止
        utils::window::window_state().set_visible(false);
        // 没有托盘就没有唤回入口：窗口关掉后必须退出，否则进程会变成一个看不见的残留
        if remaining == 0 && !tray::ready(cx) {
            log::info!("[退出] 没有托盘入口, 关闭窗口后退出");
            cx.quit();
        }
    })
    .detach();

    // 退出流程不可中断，日志必须在这里主动落盘：之后的写入不再有机会跑到磁盘。
    cx.on_app_quit(|_| {
        log::info!("[退出] 应用准备退出");
        logging::flush();
        async {}
    })
    .detach();

    log::info!("[启动] 退出模式: 仅显式退出");
}
