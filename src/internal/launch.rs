use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use anyhow::{anyhow, Context, Result};
use clap::builder::Str;
use qapi::{qmp, Qmp};
use crate::internal::helpers::get_os_assigned_port;
use crate::internal::types::{AppPackIndexFile, AppPackLocalSettings, InstalledAppPackEntry};



fn spawn_freerdp(rdp_port: &str, app_installed: &InstalledAppPackEntry) -> Result<Child> {
    let mut freerdp_command = app_installed.freerdp_command.clone();
    freerdp_command = freerdp_command.replace("$RDP_PORT", &rdp_port);
    let freerdp_command = format!("xfreerdp3 {}", freerdp_command);
    println!("Launching freerdp: {}", freerdp_command);
    let child = Command::new("bash")
        .arg("-c")
        .arg(freerdp_command)
        .spawn()
        .context("Failed to launch freerdp")?;

    Ok(child)
}

fn connect_to_appack_socket_and_launch_rdp(appack_socket_path: &Path, app_installed: &InstalledAppPackEntry) -> Result<()> {
    println!("Client: Connecting to server socket...");

    let mut stream = UnixStream::connect(appack_socket_path)?;
    println!("Client: Connected!");

    // Read server startup message (2 bytes = u16)
    let mut rdp_port = [0u8; 2];
    stream.read_exact(&mut rdp_port)?;
    let rdp_port = u16::from_le_bytes(rdp_port);

    println!("Client: Received RDP port value: {}", rdp_port);

    spawn_freerdp(&rdp_port.to_string(), app_installed)?.wait()?;

    println!("Client: Done. Disconnecting...");

    // Drop the socket to disconnect
    drop(stream);

    println!("Client: Disconnected");
    Ok(())
}


fn appack_server_logic(socket_path: &Path) -> std::io::Result<(Arc<AtomicUsize>, Sender<()>, JoinHandle<()>)> {
    let client_count = Arc::new(AtomicUsize::new(0));

    // create channel in outer scope
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    // clone a sender to move into the listener thread, keep the original to return
    let tx_for_thread = shutdown_tx.clone();
    let socket_path = socket_path.to_path_buf();
    let client_count_for_thread = client_count.clone();

    let handle = thread::spawn(move || {
        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Server: Error binding socket: {}", e);
                return;
            }
        };

        // Make accept non-blocking so we can poll for a shutdown signal
        listener.set_nonblocking(true).expect("set_nonblocking failed");
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
                        // Send initial bytes (ignore errors for brevity)
                        let _ = stream.write_all(&[1u8, 0u8]);

                        let mut buf = [0u8; 1];
                        match stream.read_exact(&mut buf) {
                            Ok(_) => {
                                // shouldn't happen; client shouldn't send
                            }
                            Err(ref e) if e.kind() == ErrorKind::UnexpectedEof
                                || e.kind() == ErrorKind::ConnectionReset =>
                                {
                                    // client disconnected gracefully
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


// fn appack_server_logic(socket_path: &Path) -> Result<Arc<AtomicUsize>> {
//     let client_count_server = Arc::new(AtomicUsize::new(0));
//     let client_count_clone = client_count_server.clone();
//
//     let socket_path = socket_path.to_path_buf();
//     std::thread::spawn(move || {
//         let listener = match std::os::unix::net::UnixListener::bind(socket_path) {
//             Ok(l) => l,
//             Err(e) => {
//                 eprintln!("Server: Error binding socket: {}", e);
//                 return;
//             }
//         };
//
//         println!("Server: Listening for external RDP clients...");
//
//         for stream_result in listener.incoming() {
//             match stream_result {
//                 Ok(mut stream) => {
//                     // Increment the count immediately upon connection
//                     client_count_server.fetch_add(1, Ordering::SeqCst);
//                     println!("Server: New client connected. Count: {}", client_count_server.load(Ordering::SeqCst));
//
//                     let client_count_handler = client_count_server.clone();
//                     stream.write_all(&[1u8, 0u8]); // TODO handle error
//                     std::thread::spawn(move || {
//                         let mut buf = [0; 1];
//                         match stream.read_exact(&mut buf) {
//                             Ok(_) => {/* Should not happen, client doesn't send data */},
//                             Err(ref e) if e.kind() == ErrorKind::UnexpectedEof || e.kind() == ErrorKind::ConnectionReset => {
//                                 // Client disconnected gracefully
//                             },
//                             Err(e) => {
//                                 eprintln!("Server Handler: Error reading from socket: {}", e);
//                             }
//                         }
//
//                         client_count_handler.fetch_sub(1, Ordering::SeqCst);
//                         let curr_conns = client_count_handler.load(Ordering::SeqCst);
//                         println!("Server Handler: Client disconnected. Count: {}", curr_conns);
//                         if curr_conns == 0 {
//
//                         }
//                     });
//                 }
//                 Err(e) => {
//                     eprintln!("Server: Error accepting connection: {}", e);
//                     break;
//                 }
//             }
//         }
//     });
//
//     Ok(client_count_clone)
// }

pub fn launch(settings: &AppPackLocalSettings, id: String, version: Option<String>, rdp_args: Option<String>) -> Result<()> {
    let all_installed = settings.get_installed().context("Failed to get installed app packs")?;
    let app_installed = all_installed.installed.into_iter().find(|i| i.id == id);
    let app_installed = app_installed.context("App pack not installed")?;
    let app_installed_home = settings.get_app_home_dir(&app_installed);
    let qmp_socket_path = app_installed_home.join("qmp-appack.sock");
    let appack_socket_path = app_installed_home.join("appack.sock");

    match connect_to_appack_socket_and_launch_rdp(&appack_socket_path, &app_installed) {
        Ok(_) => {
            return Ok(());
        },
        Err(e) => {
            println!("Failed to connect to appack socket, starting server: {}", e);
        }
    }

    let free_port = get_os_assigned_port()?;
    let local_image_file_path = app_installed_home.join(&app_installed.image);

    let mut qemu_command_str = app_installed.qemu_command.clone();
    qemu_command_str = qemu_command_str.replace("$RDP_PORT", &free_port.to_string());
    qemu_command_str = qemu_command_str.replace("$IMAGE_FILE_PATH", local_image_file_path.to_str().unwrap());

    println!("QEMU command -> {}", qemu_command_str);

    let mut qemu_command = Command::new("bash");
    qemu_command
        .arg("-c")
        .arg(format!("qemu-system-x86_64 {} -loadvm appack-init", qemu_command_str));
    let mut qemu_child = qemu_command.spawn()?;

    // Start a named socket and listen on it
    let (_, _, handle) = appack_server_logic(&appack_socket_path)?;

    match connect_to_appack_socket_and_launch_rdp(&appack_socket_path, &app_installed) {
        Ok(_) => {},
        Err(e) => {
            println!("Failed to connect to appack socket as same process {}", e);
        }
    }

    handle.join().map_err(|e| anyhow!("Could not join handle: {e:?}"))?;

    println!("All RDP sessions finished. Killing QEMU.");

    // Send a QMP message to destroy VM
    let qmp_stream = UnixStream::connect(qmp_socket_path).context("Failed to connect to QMP socket")?;
    let mut qmp = Qmp::from_stream(&qmp_stream);

    qmp.handshake().context("Failed to connect to QMP socket")?;
    match qmp.execute(&qmp::quit {}) {
        Ok(_) => {
            qemu_child.wait()?;
        },
        Err(e) => {
            println!("Failed to execute quit QMP: {}", e);
            qemu_child.kill().context("Failed to kill Qemu process")?;
        }
    };

    println!("Qemu exited");
    
    Ok(())
}