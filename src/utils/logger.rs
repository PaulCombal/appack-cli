// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2025 Paul <abonnementspaul (at) gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::fmt::Debug;

#[cfg(debug_assertions)]
pub fn log_debug<T: Debug>(message: T) {
    use anyhow::Context;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::Path;

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

#[cfg(not(debug_assertions))]
pub fn log_debug<T: Debug>(_message: T) {
    // No logs in prod
}
