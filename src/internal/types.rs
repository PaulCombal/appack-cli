use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

#[derive(Debug, Deserialize, Serialize)]
pub struct InstalledAppPackEntry {
    pub id: String,
    pub version: String,
    pub name: String,
    pub image: String,
    pub description: Option<String>,
    pub desktop_entries: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InstalledAppPacks {
    #[serde(default)]
    pub installed: Vec<InstalledAppPackEntry>,
}

#[derive(Debug)]
pub struct AppPackLocalSettings {
    pub images_folder: Rc<Path>,
    pub save_folder: Rc<Path>,
    pub qemu_uri: Rc<str>,
    pub installed_file: Rc<Path>,
}

impl Default for AppPackLocalSettings {
    fn default() -> Self {
        let snap_home = std::env::var("SNAP_USER_COMMON").unwrap_or("/etc/appack".to_string());
        let snap_home = PathBuf::from(snap_home);
        Self {
            images_folder: Rc::from(snap_home.join("images")),
            save_folder: Rc::from(snap_home.join("save")),
            qemu_uri: Rc::from("qemu:///system"),
            installed_file: Rc::from(snap_home.join("installed.yaml")),
        }
    }
}

impl AppPackLocalSettings {
    pub fn from_env() -> Self {
        let mut tmp = Self::default();
        if let Ok(appack_home) = std::env::var("APPACK_HOME") {
            let home = PathBuf::from(appack_home);
            tmp.installed_file = Rc::from(home.join("installed.yaml"));
        }

        if let Ok(libvirt_default_uri) = std::env::var("LIBVIRT_DEFAULT_URI") {
            tmp.qemu_uri = Rc::from(libvirt_default_uri);
        }
        tmp
    }
}

#[derive(Debug, Deserialize)]
pub struct AppPackIndexFile {
    pub name: String,
    pub id: String,
    pub version: String,
    pub state: String,
    pub image: String,
    pub description: Option<String>,
    pub snapshot: AppPackIndexSnapshotMode,
    pub readme: ReadmeConfiguration,
    pub base_command: String,
    pub install_append: String,
    pub configure_append: String,
    pub freerdp_command: String,
}

#[derive(Debug, Deserialize)]
pub enum AppPackIndexSnapshotMode {
    OnClose,
    Never
}

// Should we let the variables be replaced via the environment instead? Probably.
impl AppPackIndexFile {
    pub fn get_boot_install_command(&self) -> Command {
        let full_command = format!("{} {}", self.base_command, self.install_append);
        let full_command = full_command.replace("$IMAGE_FILE_PATH", &self.image);

        println!("Full boot install {}", full_command);

        let mut command = Command::new("bash");
        command.arg("-c").arg(format!("qemu-system-x86_64 {}", full_command));
        command
    }

    pub fn get_boot_configure_command(&self, rdp_port: u16) -> Command {
        let full_command = format!("{} {}", self.base_command, self.configure_append);
        let full_command = full_command.replace("$IMAGE_FILE_PATH", &self.image);
        let full_command = full_command.replace("$RDP_PORT", &rdp_port.to_string());

        println!("Full boot configure {}", full_command);

        let mut command = Command::new("bash");
        command.arg("-c").arg(format!("qemu-system-x86_64 {}", full_command));
        command
    }

    pub fn get_rdp_configure_command(&self, rdp_port: u16) -> Command {
        let full_command = self.freerdp_command.clone();
        let full_command = full_command.replace("$RDP_PORT", &rdp_port.to_string());

        println!("Full RDP configure {}", full_command);

        let mut command = Command::new("bash");
        command.arg("-c").arg(format!("xfreerdp3 {}", full_command));
        command
    }
}

#[derive(Debug, Deserialize)]
pub struct ReadmeConfiguration {
    #[serde(default = "default_readme_folder")]
    folder: String,
    #[serde(default = "default_readme_index")]
    index: String,
}

fn default_readme_folder() -> String {
    "readme".to_string()
}

fn default_readme_index() -> String {
    "README.md".to_string()
}
