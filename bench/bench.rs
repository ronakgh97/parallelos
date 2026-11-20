use aes_gcm::Nonce;
use aes_gcm::aead::{Aead, Generate, Key, KeyInit};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use parallelos::pool::WorkerPool;
use parallelos::tasks::TaskCost;
use rayon::prelude::*;
use sha2::Digest;
use std::hint::black_box;
use std::sync::LazyLock;
use std::time::Instant;

static SHARED_AES_KEY: LazyLock<aes_gcm::Aes256Gcm> = LazyLock::new(|| {
    let key_material = Key::<aes_gcm::Aes256Gcm>::generate();
    aes_gcm::Aes256Gcm::new(&key_material)
});

static SHARED_PAYLOAD: LazyLock<Box<[u8]>> = LazyLock::new(|| {
    let mut p = vec![];
    for _ in 0..448667 {
        p.extend_from_slice(
            format!(
                "user=u{:05}, action={:?}, port={}, ip=10.{}.{}.{}, status_code={}\n",
                fastrand::u64(..100_000),
                fastrand::choice(["login", "logout", "view"]),
                fastrand::u16(..65535),
                fastrand::u8(..255),
                fastrand::u8(..255),
                fastrand::u8(..255),
                fastrand::u64(400..429)
            )
            .as_bytes(),
        );
    }
    p.into_boxed_slice()
});

#[derive(Clone, Copy)]
enum TaskKind {
    Hex,
    Sha,
    Aes,
    Zstd,
}

impl TaskKind {
    const ALL: [TaskKind; 4] = [TaskKind::Hex, TaskKind::Sha, TaskKind::Aes, TaskKind::Zstd];

    #[inline]
    fn cost(self) -> TaskCost {
        match self {
            TaskKind::Hex => TaskCost::Normal,
            TaskKind::Sha => TaskCost::Low,
            TaskKind::Aes => TaskCost::Low,
            TaskKind::Zstd => TaskCost::High,
        }
    }

    #[inline]
    fn run(self) -> u64 {
        match self {
            TaskKind::Hex => {
                let start = Instant::now();
                black_box(hex::encode(SHARED_PAYLOAD.as_ref()));
                start.elapsed().as_nanos() as u64
            }
            TaskKind::Sha => {
                let start = Instant::now();
                black_box(sha2::Sha256::digest(SHARED_PAYLOAD.as_ref()));
                start.elapsed().as_nanos() as u64
            }
            TaskKind::Aes => {
                let start = Instant::now();
                let nonce = Nonce::generate();
                black_box(&SHARED_AES_KEY.encrypt(&nonce, SHARED_PAYLOAD.as_ref()).ok());
                start.elapsed().as_nanos() as u64
            }
            TaskKind::Zstd => {
                let start = Instant::now();
                black_box(zstd::bulk::compress(SHARED_PAYLOAD.as_ref(), 8).ok());
                start.elapsed().as_nanos() as u64
            }
        }
    }
}

const WORKERS: usize = 16; // I have intel i7-14650hx, 24 cores
const TASKS: usize = 4468;
const WARMUP_TASKS: usize = TASKS / 2;

fn print_stats(
    label: &str,
    makespan: f64,
    ideal_makespan: f64,
    total_busy_sec: f64,
    workers: usize,
) {
    println!("{label}");
    println!(
        "achieved makespan: {:.3} sec\nideal makespan: {:.3} sec\ntotal busy: {:.3} sec\ntotal util: {:.2}%",
        makespan,
        ideal_makespan,
        total_busy_sec,
        total_busy_sec / (makespan * workers as f64) * 100.0
    );
    println!();
}

fn main() -> Result<()> {
    // one identical task list for both libraries
    let tasks: Vec<TaskKind> = (0..TASKS)
        .map(|_| TaskKind::ALL[fastrand::usize(..TaskKind::ALL.len())])
        .collect();

    // parallelos warmup
    {
        let pool = WorkerPool::init_with(WORKERS, false)?; // no thread pinning
        let pb_task = ProgressBar::new(WARMUP_TASKS as u64);
        println!("warming up with {WARMUP_TASKS} tasks");
        pb_task.set_style(ProgressStyle::with_template(
            " {bar:40.cyan/blue} {pos:3}/{len:3} task",
        )?);

        for _ in 0..WARMUP_TASKS {
            let task = TaskKind::ALL[fastrand::usize(..TaskKind::ALL.len())];
            pool.submit_with(move || task.run(), task.cost())?;
        }

        loop {
            let total_task_submitted = pool.total_task_completed();
            pb_task.set_position(total_task_submitted);
            if total_task_submitted >= WARMUP_TASKS as u64 {
                break;
            }
            // std::thread::yield_now();

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        pb_task.finish_and_clear();
        black_box(pool.wait_all()?);
    }
    std::thread::sleep(std::time::Duration::from_secs(1));

    let pool = WorkerPool::init_with(WORKERS, false)?;
    let pb_task = ProgressBar::new(TASKS as u64);
    pb_task.set_style(ProgressStyle::with_template(
        " {bar:40.cyan/blue} {pos:3}/{len:3} task",
    )?);

    println!(
        "running {TASKS} tasks with {WORKERS} parallelos workers (payload {} MB)",
        SHARED_PAYLOAD.len() / (1 << 20)
    );

    let mut handles = Vec::with_capacity(TASKS);
    let start = Instant::now();
    let submit_start = Instant::now();
    for &task in &tasks {
        handles.push(pool.submit_with(move || task.run(), task.cost())?);
    }
    let submit_time = submit_start.elapsed().as_secs_f64();
    println!("time taken to submit tasks: {submit_time}s\n");

    loop {
        let total_task_submitted = pool.total_task_completed();
        pb_task.set_position(total_task_submitted);
        if total_task_submitted >= TASKS as u64 {
            break;
        }
        // std::thread::yield_now();

        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    pb_task.finish_and_clear();
    black_box(pool.wait_all()?); // blocks until all tasks finished
    let makespan_0 = start.elapsed().as_secs_f64();

    let worker_states = pool.state();
    println!(
        "{:>5} {:>7} {:>11} {:>8}",
        "worker", "tasks", "busy", "util"
    );
    println!("{}", "-".repeat(36));
    for (i, w) in worker_states.iter().enumerate() {
        println!(
            "{:>5} {:>7} {:>11.2}s {:>8.2}%",
            i,
            w.tasks_executed,
            w.tasks_exec_time / 1_000_000_000.0,
            w.tasks_exec_time / (makespan_0 * 1_000_000_000.0) * 100.0,
        );
    }
    println!("{}", "-".repeat(36));

    let mut hex_time = 0.0;
    let mut sha_time = 0.0;
    let mut aes_time = 0.0;
    let mut zstd_time = 0.0;
    for (task, handle) in tasks.iter().zip(handles) {
        let elapsed = handle.wait()?.0 as f64;
        match task {
            TaskKind::Hex => hex_time += elapsed,
            TaskKind::Sha => sha_time += elapsed,
            TaskKind::Aes => aes_time += elapsed,
            TaskKind::Zstd => zstd_time += elapsed,
        }
    }

    let total_worker_busy_time = worker_states.iter().map(|w| w.tasks_exec_time).sum::<f64>();
    println!(
        "hex: {:.3} sec ({:.2}%) \nsha: {:.3} sec ({:.2}%) \naes: {:.3} sec ({:.2}%) \nzstd: {:.3} sec ({:.2}%)",
        hex_time / 1_000_000_000.0,
        hex_time / (total_worker_busy_time) * 100.0,
        sha_time / 1_000_000_000.0,
        sha_time / (total_worker_busy_time) * 100.0,
        aes_time / 1_000_000_000.0,
        aes_time / (total_worker_busy_time) * 100.0,
        zstd_time / 1_000_000_000.0,
        zstd_time / (total_worker_busy_time) * 100.0
    );

    println!();
    print_stats(
        "parallelos",
        makespan_0,
        total_worker_busy_time / 1_000_000_000.0 / WORKERS as f64,
        total_worker_busy_time / 1_000_000_000.0,
        WORKERS,
    );
    drop(pool);

    std::thread::sleep(std::time::Duration::from_secs(1));

    println!("running rayon for comparison with same config");

    // rayon warmup
    {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(WORKERS)
            .build()
            .expect("failed to build rayon thread pool");

        let warmup: Vec<TaskKind> = (0..WARMUP_TASKS)
            .map(|_| TaskKind::ALL[fastrand::usize(..TaskKind::ALL.len())])
            .collect();

        pool.install(|| {
            warmup.into_par_iter().for_each(|t| {
                black_box(t.run());
            });
        });
    }
    std::thread::sleep(std::time::Duration::from_secs(1));

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(WORKERS)
        .build()
        .expect("failed to build rayon thread pool");

    let start = Instant::now();
    let results: Vec<(TaskKind, u64)> =
        pool.install(|| tasks.par_iter().map(|t| (*t, t.run())).collect());
    let makespan_1 = start.elapsed().as_secs_f64();

    let mut hex_time = 0.0;
    let mut sha_time = 0.0;
    let mut aes_time = 0.0;
    let mut zstd_time = 0.0;

    for (kind, elapsed) in results {
        match kind {
            TaskKind::Hex => hex_time += elapsed as f64,
            TaskKind::Sha => sha_time += elapsed as f64,
            TaskKind::Aes => aes_time += elapsed as f64,
            TaskKind::Zstd => zstd_time += elapsed as f64,
        }
    }

    let total_busy = hex_time + sha_time + aes_time + zstd_time;

    println!(
        "hex: {:.3} sec ({:.2}%) \nsha: {:.3} sec ({:.2}%) \naes: {:.3} sec ({:.2}%) \nzstd: {:.3} sec ({:.2}%)",
        hex_time / 1_000_000_000.0,
        hex_time / total_busy * 100.0,
        sha_time / 1_000_000_000.0,
        sha_time / total_busy * 100.0,
        aes_time / 1_000_000_000.0,
        aes_time / total_busy * 100.0,
        zstd_time / 1_000_000_000.0,
        zstd_time / total_busy * 100.0
    );

    println!();
    print_stats(
        "rayon",
        makespan_1,
        total_busy / 1_000_000_000.0 / WORKERS as f64,
        total_busy / 1_000_000_000.0,
        WORKERS,
    );

    Ok(())
}
