use serde::Deserialize;
// use std::fmt::{Display, Formatter};
use std::rc::Rc;

// #[derive(Debug)]
// pub enum AppPackError {
//     FileReadError(std::io::Error),
//     InstallationFailed { reason: String },
// }
//
// impl Display for AppPackError {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             AppPackError::FileReadError(io_err) => write!(f, "Error reading file: {io_err}"),
//             AppPackError::InstallationFailed { reason } => write!(f, "Installation failed: {reason}"),
//         }
//     }
// }
//
// impl Error for AppPackError {}

// I'm probably too stupid to find the values in the crate
// pub enum VirDomainState {
//     VirDomainNoState = 0,
//     VirDomainRunning = 1,
//     VirDomainBlocked = 2,
//     VirDomainPaused = 3,
//     VirDomainShutdown = 4,
//     VirDomainShutoff = 5,
//     VirDomainCrashed = 6,
//     VirDomainPmSuspended = 7,
// }



pub struct AppPackLocalSettings {
    pub images_folder: Rc<str>,
    pub save_folder: Rc<str>,
    pub qemu_uri: Rc<str>,
}

impl Default for AppPackLocalSettings {
    fn default() -> Self {
        Self {
            images_folder: Rc::from("/var/lib/libvirt/images"),
            save_folder: Rc::from("/var/lib/libvirt/qemu/save"),
            qemu_uri: Rc::from("qemu:///system"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AppPackIndexFile {
    pub name: String,
    pub version: String,
    pub state: String,
    pub image: String,
    pub domain: String,
    pub description: Option<String>,
    pub readme: ReadmeConfiguration,
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
