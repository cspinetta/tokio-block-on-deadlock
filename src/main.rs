use tokio_block_on_deadlock::deadlock_repro;

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    let use_tokio_driver = std::env::args().any(|a| a == "--fixed");
    deadlock_repro(use_tokio_driver).await;
}
