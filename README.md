# minimal-executor
This is an async executor tailored from futures-executor. It is meant to be as overheadless as possible.

Can be used without std
```toml
minimal-executor = { version = "0.3.0", default-features = false }
```
# Basic usage
You can use minimal-executor in three ways:
`LocalPool`, `poll_fn` and `poll_on`. They are almost the same as those in `futures`, but lighter.
```rust
fn run_until_single_future() {
    let mut cnt = 0;

    {
        let mut pool = LocalPool::new();
        let fut = lazy(|_| {
            cnt += 1;
        });
        pool.spawn(fut.boxed_local());
        pool.poll_once();
    }

    assert_eq!(cnt, 1);
}

```