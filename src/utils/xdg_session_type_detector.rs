use std::env;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    Wayland,
    X11,
    Unknown,
}

pub fn get_display_server() -> DisplayServer {
    static DISPLAY_SERVER_CACHE: OnceLock<DisplayServer> = OnceLock::new();

    *DISPLAY_SERVER_CACHE.get_or_init(|| {
        match env::var("XDG_SESSION_TYPE") {
            Ok(val) => match val.to_lowercase().as_str() {
                "wayland" => DisplayServer::Wayland,
                "x11" => DisplayServer::X11,
                _ => DisplayServer::Unknown,
            },
            Err(_) => DisplayServer::Unknown,
        }
    })
}

// wlfreerdp3 has been deprecated. XWayland will be used for quite some time still.
// They are working on a version using SDL. Might catch up later with that
pub fn get_freerdp_executable() -> &'static str {
    match get_display_server() {
        // DisplayServer::Wayland => "wlfreerdp3",
        DisplayServer::Wayland => "xfreerdp3",
        _ => "xfreerdp3",
    }
}