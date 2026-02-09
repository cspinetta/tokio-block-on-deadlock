use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn deadlock_repro(use_tokio_driver: bool) {
    log("entered deadlock_repro() inside Tokio runtime");

    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(3));
        log("[watchdog] if you're reading this, the program is probably hung as expected");
    });

    let lock = Arc::new(RwLock::new(()));

    if use_tokio_driver {
        log("calling sync_mock_callback_driven_by_tokio() (expected NOT to hang)");
        sync_mock_callback_driven_by_tokio(lock);
    } else {
        log("calling sync_mock_callback() (expected to hang)");
        sync_mock_callback(lock);
    }

    log("returned from sync_mock_callback (you should NOT see this in the deadlock case)");

    log("done");
}

fn sync_mock_callback(lock: Arc<RwLock<()>>) {
    log("entered sync_mock_callback() (sync context)");

    futures::executor::block_on(async move {
        log("inside futures::executor::block_on future");

        log("1) acquiring first read lock");
        let _g1 = lock.read().await;
        log("1) acquired first read lock");
        drop(_g1);

        log(
            "2) consuming cooperative budget (forcing future yields) — usually ~128 units, implementation detail",
        );
        for i in 0..10_000 {
            tokio::task::coop::consume_budget().await;
            if i % 20 == 0 {
                log(&format!("2) still consuming budget (i = {i})"));
            }
        }
        log("2) finished consuming budget");

        log("3) acquiring second read lock (this is where it may hang)");
        let _g2 = lock.read().await;

        log("3) acquired second read lock (if you see this, it didn't reproduce)");
        drop(_g2);
    });

    log("exiting sync_mock_callback() (you should NOT see this in the deadlock case)");
}

fn tokio_block_on<F: Future + Send>(fut: F) -> F::Output
where
    F::Output: Send,
{
    let handle = tokio::runtime::Handle::current();
    std::thread::scope(|s| s.spawn(|| handle.block_on(fut)).join().unwrap())
}

fn sync_mock_callback_driven_by_tokio(lock: Arc<RwLock<()>>) {
    log(
        "running async code via tokio::Handle::block_on on a separate OS thread (expected NOT to hang)",
    );

    tokio_block_on(async move {
        log("inside tokio-driven block_on future");

        let _g1 = lock.read().await;
        drop(_g1);

        for _ in 0..10_000 {
            tokio::task::coop::consume_budget().await;
        }

        let _g2 = lock.read().await;
        log("acquired second read (no deadlock)");
    });
}

fn log(msg: &str) {
    let t = std::thread::current();
    let name = t.name().unwrap_or("<unnamed>");
    eprintln!("[{name} {:?}] {msg}", t.id());
}
