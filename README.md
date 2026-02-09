# tokio-block-on-deadlock

Minimal reproducer for a surprising hang when mixing Tokio async code with `futures::executor::block_on`.

## What this shows

If you drive Tokio futures using `futures::executor::block_on` (often from a sync test/mock callback), you can deadlock even when nothing is contended.

Tokio uses **cooperative scheduling**. When a task exhausts its cooperative budget, Tokio may intentionally return `Poll::Pending` on an operation (even an uncontended `RwLock::read()`) to force the task to yield.

A normal Tokio runtime would re-poll the task and continue.  
`futures::executor::block_on` is *not* the Tokio runtime: when it sees `Pending`, it parks the OS thread and waits for a wake-up that never comes in this scenario.

## How to run

### Deadlock (expected to hang)

```bash
cargo run
```

You should see a log like this:

```
❯ cargo run
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/tokio-block-on-deadlock`
[main ThreadId(1)] entered deadlock_repro() inside Tokio runtime
[main ThreadId(1)] calling sync_mock_callback() (expected to hang)
[main ThreadId(1)] entered sync_mock_callback() (sync context)
[main ThreadId(1)] inside futures::executor::block_on future
[main ThreadId(1)] 1) acquiring first read lock
[main ThreadId(1)] 1) acquired first read lock
[main ThreadId(1)] 2) consuming cooperative budget (forcing future yields) — usually ~128 units, implementation detail
[main ThreadId(1)] 2) still consuming budget (i = 0)
[main ThreadId(1)] 2) still consuming budget (i = 20)
[main ThreadId(1)] 2) still consuming budget (i = 40)
[main ThreadId(1)] 2) still consuming budget (i = 60)
[main ThreadId(1)] 2) still consuming budget (i = 80)
[main ThreadId(1)] 2) still consuming budget (i = 100)
[main ThreadId(1)] 2) still consuming budget (i = 120)
[<unnamed> ThreadId(3)] [watchdog] if you're reading this, the program is probably hung as expected
^C

```

Then no further progress.

The watchdog prints after a few seconds.

### Fixed version (expected NOT to hang)

This drives the async code using `tokio::runtime::Handle::block_on` on a separate OS thread.

You should see a log like this:

```
❯ cargo run -- --fixed
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running `target/debug/tokio-block-on-deadlock --fixed`
[main ThreadId(1)] entered deadlock_repro() inside Tokio runtime
[main ThreadId(1)] calling sync_mock_callback_driven_by_tokio() (expected NOT to hang)
[main ThreadId(1)] running async code via tokio::Handle::block_on on a separate OS thread (expected NOT to hang)
[<unnamed> ThreadId(4)] inside tokio-driven block_on future
[<unnamed> ThreadId(4)] acquired second read (no deadlock)
[main ThreadId(1)] returned from sync_mock_callback (you should NOT see this in the deadlock case)
[main ThreadId(1)] done
```

And the execution completes normally. No deadlock, no hang-up.

## Takeaways

- `Poll::Pending` does not always mean "waiting on a resource".
- Tokio futures assume they are driven by a Tokio runtime.
- Avoid mixing `futures::executor::block_on` with Tokio tasks.
- If you must call async from sync, prefer `Handle::block_on` on a separate OS thread (or redesign the boundary).

## License

MIT — see [LICENSE](./LICENSE).

If you've run into similar behavior or have additional insights, feel free to open an issue.
