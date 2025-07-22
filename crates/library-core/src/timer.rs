use crate::core::AnyResult;
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{Mutex, Notify};
use tokio::time::interval;

type BoxedTask =
Box<dyn FnMut() -> Pin<Box<dyn Future<Output=AnyResult<()>> + Send>> + Send + Sync>;

pub struct Timer {
    name: String,
    task: Mutex<BoxedTask>,
    interval: Duration,
    notify: Notify,
}

impl Timer {
    pub fn new<F, R>(name: String, interval: Duration, mut task: F) -> Arc<Self>
    where
        F: FnMut() -> R + Send + Sync + 'static,
        R: Future<Output=AnyResult<()>> + Send + 'static,
    {
        let boxed_task: BoxedTask = Box::new(move || {
            let fut = task();
            Box::pin(fut)
        });

        let executor = Arc::new(Self {
            name,
            task: Mutex::new(boxed_task),
            interval,
            notify: Notify::new(),
        });

        let cloned = executor.clone();
        tokio::spawn(async move {
            cloned.run().await;
        });

        executor
    }

    async fn run(self: Arc<Self>) {
        let mut interval = interval(self.interval);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.execute().await;
                }
                _ = self.notify.notified() => {
                    self.execute().await;
                }
            }
        }
    }

    async fn execute(&self) {
        let mut task = self.task.lock().await;

        let result = AssertUnwindSafe((task)()).catch_unwind().await;

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => log::error!("[{}] 任务执行失败: {}", self.name, e),
            Err(_) => log::error!("[{}] 任务发生 panic", self.name),
        }
    }

    pub fn wake(&self) {
        self.notify.notify_one();
    }
}
