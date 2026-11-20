//! # Parallelos
//!
//! Task scheduler for parallel workloads. Submit independent tasks,
//! let the pool distribute the work such that [`makespan`](https://en.wikipedia.org/wiki/Makespan) is optimal, and collect their results when needed.
//!
//! ## Example
//!
//! ```rust
//! use std::sync::Arc;
//! use std::sync::atomic::{AtomicUsize, Ordering};
//! use parallelos::pool::WorkerPool;
//!
//! fn main() {
//!     let counter = Arc::new(AtomicUsize::new(0));
//!     let counter_clone = Arc::clone(&counter);
//!     let pool = WorkerPool::init_with(4, false).unwrap();
//!     for _ in 0..1024 {
//!         let counter_clone = Arc::clone(&counter_clone);
//!         pool.submit(move || {
//!             counter_clone.fetch_add(1, Ordering::Relaxed);
//!         }).unwrap();
//!     }
//!
//!     pool.wait_all().unwrap();
//!     let count = counter.load(Ordering::Relaxed);
//!     assert_eq!(count, 1024);
//! }
//! ```
//!
//! **Experimental:** the API and scheduling behavior may change. Nested parallelism
//! is not supported; waiting on work in the same pool from within a task can deadlock.

use crate::pool::WorkerPool;
use crate::tasks::{TaskCost, TaskHandle};
use std::sync::atomic::Ordering;

pub mod pool;
pub mod tasks;
pub(crate) mod worker;

type TaskExecutor<S> = Box<dyn FnOnce(&mut S, usize) + Send + 'static>;
/// A unit of work to be executed by a worker thread
pub(crate) struct Task<S = ()> {
    /// Enum that denotes how "costly" the task is
    pub cost: TaskCost,
    /// Closure that captures the callable, state reference, and worker id that's executing it
    pub exec: TaskExecutor<S>,
}

pub(crate) static GLOBAL_WORKER_POOL: std::sync::LazyLock<WorkerPool> =
    std::sync::LazyLock::new(|| {
        WorkerPool::init().expect("Failed to initialize global worker pool")
    });

/// Initializes the global worker pool.
pub fn init_global_pool() {
    let _ = &GLOBAL_WORKER_POOL;
}

/// Returns a reference to the global worker pool.
pub fn get_global_pool() -> &'static WorkerPool {
    init_global_pool();
    &GLOBAL_WORKER_POOL
}

/// Executes two tasks in parallel, blocks and returning their results.
pub fn parallelize<A, B, RA, RB>(
    task_a: A,
    cost_a: TaskCost,
    task_b: B,
    cost_b: TaskCost,
) -> anyhow::Result<(RA, RB)>
where
    A: FnOnce() -> RA + Send + 'static,
    B: FnOnce() -> RB + Send + 'static,
    RA: Send + 'static,
    RB: Send + 'static,
{
    let pool = get_global_pool();
    let cost_a_value = cost_a.to_value();
    let cost_b_value = cost_b.to_value();

    let first = pool
        .worker_handles
        .iter()
        .min_by_key(|w| w.stats.total_task_cost.load(Ordering::Relaxed))
        .ok_or(anyhow::anyhow!("global_pool is not initialized"))?; // should never happen, since pool can't be dropped

    let second = pool
        .worker_handles
        .iter()
        .filter(|w| w.id != first.id) // pool always have at least 2 workers, since
        .min_by_key(|w| w.stats.total_task_cost.load(Ordering::Relaxed))
        .ok_or(anyhow::anyhow!("global_pool is not initialized"))?;

    // assign costly task to the least loaded worker
    if cost_a_value < cost_b_value {
        let hb = pool.submit_to_worker(first.id, task_b, cost_b)?;
        let ha = pool.submit_to_worker(second.id, task_a, cost_a)?;
        Ok((ha.wait()?.0, hb.wait()?.0))
    } else if cost_a_value > cost_b_value {
        let ha = pool.submit_to_worker(first.id, task_a, cost_a)?;
        let hb = pool.submit_to_worker(second.id, task_b, cost_b)?;
        Ok((ha.wait()?.0, hb.wait()?.0))
    } else {
        let coin_flip = fastrand::bool();
        if coin_flip {
            let ha = pool.submit_to_worker(first.id, task_a, cost_a)?;
            let hb = pool.submit_to_worker(second.id, task_b, cost_b)?;
            Ok((ha.wait()?.0, hb.wait()?.0))
        } else {
            let hb = pool.submit_to_worker(first.id, task_b, cost_b)?;
            let ha = pool.submit_to_worker(second.id, task_a, cost_a)?;
            Ok((ha.wait()?.0, hb.wait()?.0))
        }
    }
}

/// Executes a list of tasks in parallel, blocks and returning their results in the same order as the input list.
pub fn parallelize_list<T, F>(tasks: Vec<(F, TaskCost)>) -> anyhow::Result<Vec<T>>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let pool = get_global_pool();
    let mut handles = Vec::with_capacity(tasks.len());
    for (task, cost) in tasks {
        let handle = pool.submit_with(task, cost)?;
        handles.push(handle);
    }
    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(handle.wait()?.0);
    }
    Ok(results)
}

/// Spawns a task with the given cost in the global worker pool.
/// This is a convenience wrapper around `WorkerPool::submit_with` using the global pool.
pub fn spawn<F: FnOnce() + Send + 'static>(
    task: F,
    cost: TaskCost,
) -> anyhow::Result<TaskHandle<()>> {
    let pool = get_global_pool();
    pool.submit_with(task, cost)
}

/// Waits for one provided task to `finish first` and returns its result and the remaining handles.
///
/// Remaining handles keep their relative order, no tasks are canceled and handles not passed here are unaffected.
/// If several results are ready, one is chosen randomly.
/// A selected task's panic is re-raised, dropping the remaining handles during unwinding.
pub fn finish_first<T>(
    handles: impl IntoIterator<Item = TaskHandle<T>>,
) -> (anyhow::Result<(T, usize)>, Vec<TaskHandle<T>>) {
    use crate::tasks::TaskResult;
    use crossbeam::channel::Select;

    let mut handles: Vec<_> = handles.into_iter().collect();
    if handles.is_empty() {
        return (
            Err(anyhow::anyhow!("finish_first requires at least one task")),
            handles,
        );
    }

    let (index, result) = {
        let mut select = Select::default();
        for handle in &handles {
            select.recv(&handle.rx);
        }

        let operation = select.select();
        let index = operation.index();
        let result = operation.recv(&handles[index].rx);
        (index, result)
    };
    handles.remove(index);

    let result = match result {
        Ok(TaskResult::Success(value, worker_id)) => Ok((value, worker_id)),
        Ok(TaskResult::Panic(payload)) => std::panic::resume_unwind(payload),
        Err(_) => Err(anyhow::anyhow!("task {index} failed to send its result")),
    };
    (result, handles)
}

/// Consumes submitted handles and yields their results as they become ready.
///
/// Each `next()` blocks until a result is ready; empty input ends immediately.
/// If multiple results are ready, their relative order is unspecified.
/// Task panics are re-raised, just like [`TaskHandle::wait`].
/// Dropping the iterator drops remaining handles, but does not cancel their tasks.
pub fn as_completed<T>(
    handles: impl IntoIterator<Item = TaskHandle<T>>,
) -> impl Iterator<Item = anyhow::Result<(T, usize)>> {
    let mut handles: Vec<TaskHandle<T>> = handles.into_iter().collect();
    std::iter::from_fn(move || {
        // return empty iterator if no handles remain
        if handles.is_empty() {
            return None;
        }

        let (index, result) = {
            let mut select = crossbeam::channel::Select::default();
            for handle in &handles {
                select.recv(&handle.rx);
            }
            let operation = select.select();
            let index = operation.index();
            (index, operation.recv(&handles[index].rx))
        };
        handles.swap_remove(index); // remove the handle that completed
        Some(match result {
            Ok(tasks::TaskResult::Success(value, worker_id)) => Ok((value, worker_id)),
            Ok(tasks::TaskResult::Panic(payload)) => std::panic::resume_unwind(payload),
            Err(_) => Err(anyhow::anyhow!(
                "failed to recv task result, possible reasons: worker thread panicked or dropped before sending the result"
            )),
        })
    })
}

#[test]
fn lib_test() {
    init_global_pool();
    let pool = WorkerPool::init_with(4, false).unwrap();
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    let task_handles: Vec<_> = (0..100)
        .map(|_| {
            let counter_clone = std::sync::Arc::clone(&counter);
            pool.submit(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();

    for handle in task_handles {
        let _ = handle.unwrap().wait();
    }
    // pool.wait_all_tasks();

    assert_eq!(counter.load(Ordering::SeqCst), 100);

    let time_taken = std::time::Instant::now();
    parallelize(
        || {
            std::thread::sleep(std::time::Duration::from_millis(200));
        },
        TaskCost::Normal,
        || {
            std::thread::sleep(std::time::Duration::from_millis(100));
        },
        TaskCost::Normal,
    )
    .unwrap();
    let elapsed = time_taken.elapsed();
    assert!(elapsed < std::time::Duration::from_millis(225));

    // submit each task to a different worker so the scheduler cannot co-locate
    // them on one worker (where FIFO would make the 500ms task finish first)
    // finish_first is about result readiness, not scheduler placement
    let task_1 = pool
        .submit_to_worker(
            0,
            || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                44
            },
            TaskCost::Normal,
        )
        .unwrap();

    let task_2 = pool
        .submit_to_worker(
            1,
            || {
                std::thread::sleep(std::time::Duration::from_millis(200));
                97
            },
            TaskCost::Normal,
        )
        .unwrap();

    let (first, second) = finish_first([task_1, task_2]);
    assert_eq!(first.unwrap().0, 97);
    assert!(!second.is_empty());
}
