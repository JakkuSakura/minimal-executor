use std::cell::Cell;
use std::rc::Rc;
use futures::future::lazy;
use minimal_executor::{BusyLocalPool, LocalPool, NewLocalPool};
use criterion::{criterion_group, criterion_main, Criterion};

fn spawn_many_old(mut pool: LocalPool, iter: usize) {
    let cnt = Rc::new(Cell::new(0));

    for _ in 0..iter {
        let cnt = cnt.clone();
        pool.spawn(Box::pin(lazy(move |_| {
            cnt.set(cnt.get() + 1);
        })));
    }

    pool.run();

    assert_eq!(cnt.get(), iter);
}

fn spawn_many_new(mut pool: NewLocalPool, iter: usize) {
    let cnt = Rc::new(Cell::new(0));


    for _ in 0..iter {
        let cnt = cnt.clone();
        pool.spawn(Box::pin(lazy(move |_| {
            cnt.set(cnt.get() + 1);
        })));
    }

    pool.run();

    assert_eq!(cnt.get(), iter);
}

fn spawn_many_busy(mut pool: BusyLocalPool, iter: usize) {
    let cnt = Rc::new(Cell::new(0));

    for _ in 0..iter {
        let cnt = cnt.clone();
        pool.spawn(Box::pin(lazy(move |_| {
            cnt.set(cnt.get() + 1);
        })));
    }

    pool.run();

    assert_eq!(cnt.get(), iter);
}

pub fn criterion_benchmark(c: &mut Criterion) {
    for i in [2, 20, 200] {
        c.bench_function(&format!("spawn_many_old {}", i), |b|
            b.iter_with_setup(|| LocalPool::new(), |p| spawn_many_old(p, i)),
        );
        c.bench_function(&format!("spawn_many_new {}", i), |b|
            b.iter_with_setup(|| NewLocalPool::new(), |p| spawn_many_new(p, i)),
        );
        c.bench_function(&format!("spawn_many_busy {}", i), |b|
            b.iter_with_setup(|| BusyLocalPool::new(256), |p| spawn_many_busy(p, i)),
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);