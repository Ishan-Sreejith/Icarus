use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::runtime::Runtime;

/// JavaScript-style event loop for async operations
#[allow(dead_code)]
pub struct EventLoop {
    runtime: Runtime,
    task_queue: VecDeque<Pin<Box<dyn Future<Output = ()> + Send>>>,
}

#[allow(dead_code)]
impl EventLoop {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        EventLoop {
            runtime,
            task_queue: VecDeque::new(),
        }
    }

    pub fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.task_queue.push_back(Box::pin(future));
    }

    pub fn run(&mut self) {
        while let Some(task) = self.task_queue.pop_front() {
            self.runtime.block_on(task);
        }
    }

    pub fn run_async<F, T>(&self, future: F) -> T
    where
        F: Future<Output = T>,
    {
        self.runtime.block_on(future)
    }
}

/// Task handle for async operations
#[allow(dead_code)]
pub struct Task<T> {
    inner: Pin<Box<dyn Future<Output = T> + Send>>,
}

#[allow(dead_code)]
impl<T> Task<T> {
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        Task {
            inner: Box::pin(future),
        }
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.as_mut().poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_loop() {
        let mut event_loop = EventLoop::new();

        event_loop.spawn(async {
            println!("Task 1");
        });

        event_loop.spawn(async {
            println!("Task 2");
        });

        event_loop.run();
    }
}
