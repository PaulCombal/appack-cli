use crate::internal::types::AppPackIndexFile;
use anyhow::{Result, anyhow};
use std::io::Read;
use std::net::{Ipv4Addr, TcpListener};
use std::path::Path;
use std::process::Command;

fn create_image(path: &Path) -> Result<()> {
    Command::new("qemu-img")
        .arg("create")
        .arg("-f")
        .arg("qcow2")
        .arg(path)
        .arg("32G")
        .status()
        .map_err(|e| anyhow!("Failed to create image: {}", e))?;

    Ok(())
}

fn read_config(path: &Path) -> Result<AppPackIndexFile> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| anyhow!("Unable to open config file at {}: {}", path.display(), e))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| anyhow!("Unable to read config file contents: {}", e))?;

    serde_yaml::from_slice(&buffer).map_err(|e| anyhow!("Invalid YAML format in file: {:?}", e))
}

// Yes there is a race condition here. This is a problem for later.
fn get_os_assigned_port() -> Result<u16> {
    let listener = TcpListener::bind(format!("{}:0", Ipv4Addr::LOCALHOST))?;
    let port = listener.local_addr()?.port();
    Ok(port)
}

pub fn creator_new() -> Result<()> {
    let assets_path_str =
        std::env::var("SNAP").map_err(|e| anyhow!("Failed to get assets path: {}", e))?;
    let assets_path = Path::new(&assets_path_str).join("assets");
    std::fs::create_dir("AppPack")
        .map_err(|e| anyhow!("Failed to create AppPack directory: {}", e))?;
    std::fs::create_dir("AppPack/readme")
        .map_err(|e| anyhow!("Failed to create readme directory: {}", e))?;
    std::fs::create_dir("AppPack/desktop")
        .map_err(|e| anyhow!("Failed to create desktop directory: {}", e))?;

    std::fs::copy(
        assets_path.join("creator").join("README.md"),
        "AppPack/readme/README.md",
    )?;
    std::fs::copy(
        assets_path.join("creator").join("AppPack.yaml"),
        "AppPack/AppPack.yaml",
    )?;

    create_image(Path::new("AppPack/image.qcow2"))?;

    Ok(())
}

pub fn creator_boot_install() -> Result<()> {
    let config = read_config(Path::new("AppPack.yaml"))?;

    let mut command = config.get_boot_install_command();

    command.status()?;

    Ok(())
}

pub fn creator_boot() -> Result<()> {
    let config = read_config(Path::new("AppPack.yaml"))?;
    let free_port = get_os_assigned_port()?;

    let mut qemu_command = config.get_boot_configure_command(free_port);
    let mut qemu_child = qemu_command.spawn()?;

    let mut rdp_command = config.get_rdp_configure_command(free_port);

    for i in 1..5 {
        println!("Trying to connect to RDP on port {}...", free_port);

        match rdp_command.status() {
            Ok(status) => {
                if status.success() {
                    println!("RDP was successful");
                    break;
                }

                println!("RDP failed: {status}. Retrying in 5 seconds ({i}/5)");
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
            Err(e) => {
                println!("Failed to start RDP process for port {} ({e:?}). Retrying in 5 seconds ({i}/5)", free_port);
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        }
    }

    qemu_child.wait()?;
    println!("Qemu exited");

    Ok(())
}
