use std::cell::RefCell;

use codas_flow::{
    stage::{Proc, Stage},
    Flow,
};
use criterion::{criterion_group, criterion_main, Criterion};
use tokio::sync::{broadcast, mpsc};

const BUFFER_SIZE: usize = 1024;

fn channels(c: &mut Criterion) {
    let mut group = c.benchmark_group("Channels");
    group.throughput(criterion::Throughput::Elements(1));

    group.bench_function("Many(1):1 Flow (Subscriber); Move->Read", |b| {
        let i = RefCell::new(0);
        let (pubs, [mut subs]) = Flow::<TestStruct>::new(BUFFER_SIZE);
        let pubs = RefCell::new(pubs);

        // Spawn event handler in a loop.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.spawn(async move {
            let mut next_i = 0;

            loop {
                let data = subs.next().await.expect("value");
                assert_eq!(next_i, data.value as u64);
                next_i += 1;
            }
        });

        // Publish lots of events.
        b.to_async(runtime).iter(|| async {
            let mut pubs = pubs.borrow_mut();
            let mut next = pubs.next().await.expect("next");
            let mut i = i.borrow_mut();
            next.value = *i;
            drop(next);
            *i += 1;
        });
    });

    group.bench_function(
        "Many(1):Many(1) Flow (Stage); Move->Read (Crate Yield)",
        |b| {
            let i = RefCell::new(0);
            let (pubs, [subs]) = Flow::<TestStruct>::new(BUFFER_SIZE);
            let pubs = RefCell::new(pubs);

            // Prepare event handler.
            let mut stage = Stage::from(subs);
            let mut next_i = 0;
            stage.add_proc(move |_: &mut Proc, data: &TestStruct| {
                assert_eq!(next_i, data.value as u64);
                next_i += 1;
            });

            // Spawn event handler in a loop.
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.spawn(stage.proc_loop());

            // Publish lots of events.
            b.to_async(runtime).iter(|| async {
                let mut pubs = pubs.borrow_mut();
                let mut next = pubs.next().await.expect("next");
                let mut i = i.borrow_mut();
                next.value = *i;
                drop(next);
                *i += 1;
            });
        },
    );

    group.bench_function(
        "Many(1):Many(1) Flow (Stage); Move->Read (Tokio Yield)",
        |b| {
            let i = RefCell::new(0);
            let (pubs, [subs]) = Flow::<TestStruct>::new(BUFFER_SIZE);
            let pubs = RefCell::new(pubs);

            // Prepare event handler.
            let mut stage = Stage::from(subs);
            let mut next_i = 0;
            stage.add_proc(move |_: &mut Proc, data: &TestStruct| {
                assert_eq!(next_i, data.value as u64);
                next_i += 1;
            });

            // Spawn event handler in a loop.
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.spawn(stage.proc_loop_with_waiter(tokio::task::yield_now));

            // Publish lots of events.
            b.to_async(runtime).iter(|| async {
                let mut pubs = pubs.borrow_mut();
                let mut next = pubs.next().await.expect("next");
                let mut i = i.borrow_mut();
                next.value = *i;
                drop(next);
                *i += 1;
            });
        },
    );

    group.bench_function("Many(1):1 Tokio (MPSC); Move->Take", |b| {
        let i = RefCell::new(0);
        let (tx, mut rx) = mpsc::channel::<TestStruct>(BUFFER_SIZE);

        // Spawn event handler in a loop.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.spawn(async move {
            let mut next_i = 0;

            loop {
                let data = rx.recv().await.expect("value");
                assert_eq!(next_i, data.value as u64);
                next_i += 1;
            }
        });

        // Publish lots of events.
        b.to_async(runtime).iter(|| async {
            tx.send(TestStruct { value: *i.borrow() }).await.unwrap();
            *i.borrow_mut() += 1;
        });
    });

    group.bench_function("Many(1):Many(1) Tokio (Broadcast); Move->Clone", |b| {
        let i = RefCell::new(0);
        let (tx, mut rx) = broadcast::channel::<TestStruct>(BUFFER_SIZE);

        // Spawn event handler in a loop.
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.spawn(async move {
            let mut next_i = 0;

            loop {
                match rx.recv().await {
                    Ok(data) => {
                        assert_eq!(next_i, data.value as u64);
                        next_i += 1;
                    }
                    Err(broadcast::error::RecvError::Lagged(lag)) => next_i += lag,
                    Err(..) => unimplemented!(),
                }
            }
        });

        // Publish lots of events.
        b.to_async(runtime).iter(|| async {
            let mut i = i.borrow_mut();
            let _ = tx.send(TestStruct { value: *i }).unwrap();
            *i += 1;
        });
    });
}

// Create a new group named `benches` and
// run it with all benchmark methods.
criterion_group!(benches, channels);
criterion_main!(benches);

/// Simplistic test data structure for [`channels`].
#[derive(Debug, Default, Clone)]
struct TestStruct {
    value: i64,
}
