use crate::internal::types::AppPackLocalSettings;

pub fn print_info(settings: &AppPackLocalSettings) {
    println!("AppPack version: {}", env!("CARGO_PKG_VERSION"));
    println!("Settings: {settings:?}");
}
