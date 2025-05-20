use std::{fs::File, time::Duration};

use daemonize_me::Daemon;
use tokio::{
    runtime::Runtime,
    time::{sleep, Instant},
};

// We can't use #[tokio::main] here!
// We need to daemonize first and only then may we initialize the tokio runtime.
fn main() {
    let stdout = File::create("info.log").unwrap();
    let stderr = File::create("err.log").unwrap();
    let daemon = Daemon::new()
        .pid_file("example.pid", Some(false))
        .work_dir(".")
        .stdout(stdout)
        .stderr(stderr)
        .start();

    match daemon {
        Ok(_) => println!("Daemonized with success"),
        Err(e) => eprintln!("Error, {}", e),
    }

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let now = Instant::now();
        println!("Before async sleep, elapsed: {}", now.elapsed().as_secs());
        sleep(Duration::from_secs(5)).await;
        println!("After async sleep, elapsed: {}", now.elapsed().as_secs());
    });

    println!("Finished execution");
}
