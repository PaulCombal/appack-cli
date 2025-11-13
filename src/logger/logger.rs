use anyhow::Context;
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub fn log_debug<T: Debug>(message: T) {
    const LOG_FILE_NAME: &str = "log.txt";
    let snap_dir = std::env::var("SNAP_USER_COMMON")
        .context("Not in a Snap")
        .unwrap();
    let log_path = Path::new(&snap_dir).join(LOG_FILE_NAME);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .context("Couldn't open log file")
        .unwrap();

    let formatted_message = format!("{:?}\n", message);
    if let Err(e) = file.write_all(formatted_message.as_bytes()) {
        eprintln!("Error writing to log file: {}", e);
    }
}
