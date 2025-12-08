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

use crate::internal::helpers::{get_os_assigned_port, has_snapshot};
use crate::types::AppSnapshotTriggerMode;
use crate::types::app_installed::InstalledAppPackEntry;
use crate::types::local_settings::AppPackLocalSettings;
use crate::utils::logger::log_debug;
use crate::utils::qmp::{delete_snapshot_blocking, take_snapshot_blocking};
use anyhow::{Context, Result, anyhow};
use qapi::{Qmp, qmp};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, mpsc};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

fn to_win_escaped_path(path: &str) -> String {
    const PREFIX: &str = "\\\\tsclient\\home\\";

    if path == "" {
        return "".to_string();
    }

    let mut stripped_path = path;

    if path.starts_with("'") && path.ends_with("'") && path.len() >= 2 {
        stripped_path = &path[1..path.len() - 1];
    }

    if stripped_path.starts_with("/home/") {
        // Find the slash that comes after "/home"
        if let Some(first_slash_after_home) = stripped_path[6..].find('/') {
            let start_index = 6 + first_slash_after_home + 1;
            stripped_path = &stripped_path[start_index..];
        }
    }

    let clean_path = stripped_path.trim_start_matches('/');
    let windows_style_path = clean_path.replace('/', "\\");
    format!("{}{}", PREFIX, windows_style_path)
}

fn detect_and_replace_win_escape(argstr: &str) -> String {
    const FUNC_START: &str = "$TO_WIN_ESCAPED_PATH**";
    const FUNC_END: &str = "**";

    let mut result = String::new();
    let mut current_pos = 0;

    while let Some(start_relative_index) = argstr[current_pos..].find(FUNC_START) {
        let absolute_start = current_pos + start_relative_index;
        let arg_start = absolute_start + FUNC_START.len();
        result.push_str(&argstr[current_pos..absolute_start]);

        if let Some(end_relative_index) = argstr[arg_start..].find(FUNC_END) {
            let absolute_end = arg_start + end_relative_index;
            let unix_path_arg = &argstr[arg_start..absolute_end];
            let windows_path = to_win_escaped_path(unix_path_arg).replace(" ", "$WHITESPACE");
            result.push_str(&windows_path);
            current_pos = absolute_end + FUNC_END.len();
        } else {
            // Error handling: FUNC_START found but no matching FUNC_END
            result.push_str(&argstr[absolute_start..]);
            current_pos = argstr.len();
            break;
        }
    }

    result.push_str(&argstr[current_pos..]);

    result
}

// This is repetitive and ugly. To refactor.
fn spawn_freerdp(
    rdp_port: &str,
    app_installed: &InstalledAppPackEntry,
    rdp_args: Option<&str>,
) -> Result<Child> {
    let base = app_installed.freerdp_command.clone();
    let snap_real_home = std::env::var("SNAP_REAL_HOME")?;

    let mut full_cmd = match rdp_args {
        Some(args) => format!("{} {}", base, args),
        None => base,
    };

    full_cmd = full_cmd
        .replace("$RDP_PORT", rdp_port)
        .replace("$HOME", &snap_real_home);

    full_cmd = detect_and_replace_win_escape(&full_cmd);

    let args: Vec<String> = full_cmd
        .split_whitespace()
        .map(|s| s.replace("$WHITESPACE", " "))
        .collect();

    println!("Launching xfreerdp3 with args: {args:?}");
    log_debug("Launching xfreerdp3 with args: ");
    log_debug(&args);

    let child = Command::new("xfreerdp3")
        .args(args)
        .spawn()
        .context("Failed to launch xfreerdp3")?;

    Ok(child)
}

fn connect_to_appack_socket_and_launch_rdp(
    appack_socket_path: &Path,
    app_installed: &InstalledAppPackEntry,
    rdp_args: Option<&str>,
) -> Result<()> {
    println!("Client: Connecting to AppPack socket: {appack_socket_path:?}");

    let mut stream = match UnixStream::connect(appack_socket_path) {
        Ok(stream) => stream,
        Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
            println!("It looks like Qemu previously crashed. Cleaning up and starting server.");
            std::fs::remove_file(appack_socket_path).context("Failed to remove AppPack socket")?;
            return Err(anyhow!(e).context("Failed to connect to AppPack socket"));
        }
        Err(e) => {
            return Err(anyhow!(e).context("Failed to connect to AppPack socket"));
        }
    };

    println!("Client: Connected!");

    // Read server startup message (2 bytes = u16)
    let mut rdp_port = [0u8; 2];
    stream.read_exact(&mut rdp_port)?;
    let rdp_port = u16::from_le_bytes(rdp_port);

    println!("Client: Received RDP port value: {}", rdp_port);

    spawn_freerdp(&rdp_port.to_string(), app_installed, rdp_args)?.wait()?;

    println!("Client: Done. Disconnecting...");

    // Drop the socket to disconnect
    drop(stream);

    println!("Client: Disconnected");
    Ok(())
}

fn appack_server_logic(
    socket_path: &Path,
    rdp_port: u16,
) -> std::io::Result<(Arc<AtomicUsize>, Sender<()>, JoinHandle<()>)> {
    let client_count = Arc::new(AtomicUsize::new(0));

    // create channel in outer scope
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    // clone a sender to move into the listener thread, keep the original to return
    let tx_for_thread = shutdown_tx.clone();
    let socket_path = socket_path.to_path_buf();
    let client_count_for_thread = client_count.clone();

    println!("Launching AppPack server thread");
    let handle = thread::spawn(move || {
        println!("AppPack server thread spawned. Binding socket: {socket_path:?}");
        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Server: Error binding socket: {}", e);
                return;
            }
        };

        // Make accept non-blocking so we can poll for a shutdown signal
        listener
            .set_nonblocking(true)
            .expect("set_nonblocking failed");
        println!("Server: Listening for external RDP clients...");

        loop {
            match listener.accept() {
                Ok((mut stream, _addr)) => {
                    // Increment count immediately
                    client_count_for_thread.fetch_add(1, Ordering::SeqCst);
                    println!(
                        "Server: New client connected. Count: {}",
                        client_count_for_thread.load(Ordering::SeqCst)
                    );

                    // Clone handles for the handler thread.
                    let client_count_handler = client_count_for_thread.clone();
                    let handler_tx = tx_for_thread.clone();

                    // spawn handler thread
                    thread::spawn(move || {
                        // Send RDP port on client connection
                        match stream.write_all(rdp_port.to_le_bytes().as_slice()) {
                            Ok(_) => (),
                            Err(e) => {
                                eprintln!("Server: Error writing RDP port to client: {}", e);
                                return;
                            }
                        }

                        let mut buf = [0u8; 1];
                        match stream.read_exact(&mut buf) {
                            Ok(_) => {
                                println!(
                                    "Server: Received unexpected value from client: {}",
                                    buf[0]
                                );
                            }
                            Err(ref e)
                                if e.kind() == ErrorKind::UnexpectedEof
                                    || e.kind() == ErrorKind::ConnectionReset =>
                            {
                                println!("Server: Client disconnected gracefully");
                            }
                            Err(e) => {
                                eprintln!("Server Handler: Error reading from socket: {}", e);
                            }
                        }

                        client_count_handler.fetch_sub(1, Ordering::SeqCst);
                        let c = client_count_handler.load(Ordering::SeqCst);
                        println!("Server Handler: Client disconnected. Count: {}", c);

                        // if no clients remain, notify the listener thread
                        if c == 0 {
                            // ignore send error (receiver might have been dropped)
                            let _ = handler_tx.send(());
                        }
                    });
                }

                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // No connection right now: check for shutdown signal
                    if shutdown_rx.try_recv().is_ok() {
                        println!("Server: Shutdown requested and received. Exiting listener loop.");
                        break;
                    }
                    // small sleep to avoid busy loop
                    thread::sleep(Duration::from_millis(500));
                }

                Err(e) => {
                    eprintln!("Server: Error accepting connection: {}", e);
                    break;
                }
            }
        }

        // Clean up socket file (ignore errors)
        let _ = std::fs::remove_file(&socket_path);
        println!("Server thread exiting.");
    });

    // return the original sender so caller can optionally request shutdown
    Ok((client_count, shutdown_tx, handle))
}

pub fn launch(
    settings: &AppPackLocalSettings,
    id: String,
    version: Option<&str>,
    rdp_args: Option<&str>,
) -> Result<()> {
    let app_installed = settings
        .get_app_installed(&id, version)
        .context("Failed to get installed AppPack")?;
    let app_installed_home = settings.get_app_home_dir(&app_installed);
    let qmp_socket_path = app_installed_home.join("qmp-appack.sock");
    let appack_socket_path = app_installed_home.join("appack.sock");

    println!("Launching AppPack: {id} (version {version:?}, RDP: {rdp_args:?})");

    match connect_to_appack_socket_and_launch_rdp(&appack_socket_path, &app_installed, rdp_args) {
        Ok(_) => {
            return Ok(());
        }
        Err(e) => {
            println!("Failed to connect to appack socket, starting server: {}", e);
        }
    }

    // Wait util it's not possible to connect to the QMP socket
    // This is to handle the case when a user is trying to relaunch an appack when it's doing an OnClose snapshot
    // or shutting down
    {
        let mut notif_shown = false;
        loop {
            match UnixStream::connect(&qmp_socket_path) {
                Ok(_) => {
                    if !notif_shown {
                        notify_rust::Notification::new()
                            .summary(&format!("\"{}\" will open soon", app_installed.name))
                            .body("Please be patient while we're setting things up")
                            .show()
                            .context("Failed to show desktop notification")?;
                        notif_shown = true;
                    }

                    println!(
                        "It looks like a VM is still running for this AppPack.. Waiting for it to close"
                    );
                    thread::sleep(Duration::from_millis(300));
                }
                Err(_) => {
                    break;
                }
            }
        }
    }

    let free_port = get_os_assigned_port()?;
    let absolute_image_file_path = app_installed_home.join(&app_installed.image);

    let mut qemu_command_str = app_installed.qemu_command.clone();
    qemu_command_str = qemu_command_str.replace("$RDP_PORT", &free_port.to_string());
    qemu_command_str = qemu_command_str.replace(
        "$IMAGE_FILE_PATH",
        absolute_image_file_path.to_str().unwrap(),
    );

    match app_installed.snapshot_mode {
        // Never load any state, cold boot
        AppSnapshotTriggerMode::NeverLoad => {}

        // Always load the same startup state
        AppSnapshotTriggerMode::Never => {
            let has_init_snapshot = has_snapshot("appack-init", &absolute_image_file_path)?;
            if !has_init_snapshot {
                return Err(anyhow!("Missing snapshot 'appack-init' from image")
                    .context("The AppPack hasn't been packaged properly"));
            }

            qemu_command_str = format!("{qemu_command_str} -loadvm appack-init")
        }

        // Load the most significant or none at all
        AppSnapshotTriggerMode::OnClose => {
            let has_onclose_snapshot = has_snapshot("appack-onclose", &absolute_image_file_path)?;
            if !has_onclose_snapshot {
                let has_init_snapshot = has_snapshot("appack-init", &absolute_image_file_path)?;
                if has_init_snapshot {
                    println!(
                        "AppPack doesn't have a running state, using 'appack-init' snapshot as backup"
                    );
                    qemu_command_str = format!("{qemu_command_str} -loadvm appack-init")
                } else {
                    println!("AppPack doesn't have any live state, doing cold boot as backup");

                    notify_rust::Notification::new()
                        .summary(&format!(
                            "Launching \"{}\" for the first time",
                            app_installed.name
                        ))
                        .body("Please be patient while we're setting things up")
                        .show()
                        .context("Failed to show desktop notification")?;
                }
            } else {
                qemu_command_str = format!("{qemu_command_str} -loadvm appack-onclose")
            }
        }
    }

    println!("Starting Qemu with params: {}", qemu_command_str);
    let qemu_command_args = qemu_command_str.split_whitespace().collect::<Vec<&str>>();

    let mut qemu_command = Command::new("qemu-system-x86_64");
    qemu_command
        .current_dir(app_installed_home) // Necessary to make the qmp socket in the dir, although we could find and replace it like other vars it
        .args(qemu_command_args);
    let mut qemu_child = qemu_command.spawn()?;

    // Wait for qmp socket to be available
    loop {
        match qemu_child.try_wait() {
            // 1. Ok(None): Child is STILL RUNNING
            Ok(None) => {
                match UnixStream::connect(&qmp_socket_path) {
                    Ok(_) => {
                        break;
                    }
                    Err(e) => {
                        println!("Waiting for QMP socket connection: {}", e);
                        thread::sleep(Duration::from_millis(200));
                    }
                };
            }

            // 2. Ok(Some(status)): Child has EXITED
            Ok(Some(status)) => {
                eprintln!("QEMU process unexpectedly exited with status: {}", status);
                return Err(anyhow!("QEMU process died before QMP socket was ready.")
                    .context("Qemu failed to start. Make sure you installed AppPack with the command on the Readme (with the appropriate connections)."));
            }

            // 3. Err(e): An error occurred while trying to check the status
            Err(e) => {
                return Err(anyhow!(e).context("Error while checking QEMU status"));
            }
        }
    }

    println!("QMP socket is ready! Continuing.");

    let (_, _, handle) = appack_server_logic(&appack_socket_path, free_port)?;

    // Just wait a little bit to make sure the server thread started
    thread::sleep(Duration::from_millis(50));

    match connect_to_appack_socket_and_launch_rdp(&appack_socket_path, &app_installed, rdp_args) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to connect to appack socket as same process {}", e);
        }
    }

    handle
        .join()
        .map_err(|e| anyhow!("Could not join handle: {e:?}"))?;

    println!("All RDP sessions finished. Killing QEMU.");

    // Send a QMP message to destroy VM
    let qmp_stream = UnixStream::connect(&qmp_socket_path).context(format!(
        "Failed to connect to QMP socket ({qmp_socket_path:?})"
    ))?;
    let mut qmp = Qmp::from_stream(&qmp_stream);

    qmp.handshake().context("Failed to connect to QMP socket")?;

    match app_installed.snapshot_mode {
        AppSnapshotTriggerMode::OnClose => {
            println!(
                "App has snapshot mode OnClose, taking 'appack-onclose' snapshot before quitting"
            );

            // Wait a little bit before taking the snapshot, so the OS has time to finish the logoff
            thread::sleep(Duration::from_millis(500));

            // This can fail silently if the snapshot doesn't exist for example
            let _ = delete_snapshot_blocking(&mut qmp, "appack-onclose");
            take_snapshot_blocking(&mut qmp, "appack-onclose")?;
        }
        _ => {}
    }

    match qmp.execute(&qmp::quit {}) {
        Ok(_) => {
            qemu_child
                .wait()
                .context("Failed to wait for qemu process to exit")?;
        }
        Err(e) => {
            println!("Failed to execute quit QMP: {}", e);
            qemu_child.kill().context("Failed to kill Qemu process")?;
        }
    };

    println!("Qemu exited");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_with_username_and_file() {
        // Test case: standard path with username and file
        let path = "/home/john_doe/documents/report.pdf";
        let expected = "\\\\tsclient\\home\\documents\\report.pdf";
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_path_with_different_username() {
        // Test case: different username
        let path = "/home/dev-user/code/main.rs";
        let expected = "\\\\tsclient\\home\\code\\main.rs";
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_path_with_no_trailing_file() {
        // Test case: path is just a directory after the username
        let path = "/home/alice/Projects/";
        let expected = "\\\\tsclient\\home\\Projects\\";
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_path_is_only_root_home() {
        // Test case: path is exactly /home/{username} (edge case, result is the root share path)
        let path = "/home/bob";
        let expected = "\\\\tsclient\\home\\home\\bob";
        // NOTE: The current simple implementation relies on finding the *next* slash.
        // If the input path is exactly `/home/username`, the implementation assumes it's
        // not a valid path and doesn't strip it, leaving it as a relative path.
        // If the desired output for `/home/bob` is `\\\\tsclient\\home\\`, then the
        // function's logic needs more complexity. Sticking to the primary request:
        // /home/anyusername/ is the pattern to remove. Since there's no trailing '/',
        // the path is NOT stripped.
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_path_is_only_root_home_with_slash() {
        // Test case: path is exactly /home/{username}/ (should be stripped to empty)
        let path = "/home/bob/";
        let expected = "\\\\tsclient\\home\\";
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_path_already_stripped() {
        // Test case: path does not start with /home/
        let path = "/tmp/data/log.txt";
        let expected = "\\\\tsclient\\home\\tmp\\data\\log.txt";
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_relative_path() {
        // Test case: relative path
        let path = "data/input.csv";
        let expected = "\\\\tsclient\\home\\data\\input.csv";
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_empty_path() {
        // Test case: empty path
        let path = "";
        let expected = "";
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_path_with_leading_slash_only() {
        // Test case: just a leading slash (should result in the base path)
        let path = "/";
        let expected = "\\\\tsclient\\home\\";
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_with_space() {
        let path = "/home/dude/i have space/file.txt";
        let expected = "\\\\tsclient\\home\\i have space\\file.txt";
        assert_eq!(to_win_escaped_path(path), expected);
    }

    #[test]
    fn test_with_space_and_single_quotes() {
        let path = "'/home/dude/i have space/file.txt'";
        let expected = "\\\\tsclient\\home\\i have space\\file.txt";
        assert_eq!(to_win_escaped_path(path), expected);
    }
}
