use std::env;
use std::path::Path;

const DISABLE_DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";

/// WebKitGTK's DMABUF renderer crashes the app on Wayland with the proprietary
/// NVIDIA driver ("Error 71 (Protocol error) dispatching to Wayland display").
/// Turning it off costs some rendering speed, so it is done only for that
/// combination, and never when the user has set the variable themselves.
fn needs_dmabuf_workaround(on_wayland: bool, has_nvidia_driver: bool, already_set: bool) -> bool {
    on_wayland && has_nvidia_driver && !already_set
}

pub fn prepare_environment() {
    let on_wayland = env::var_os("WAYLAND_DISPLAY").is_some();
    let has_nvidia_driver = Path::new("/sys/module/nvidia").exists();
    let already_set = env::var_os(DISABLE_DMABUF).is_some();

    if needs_dmabuf_workaround(on_wayland, has_nvidia_driver, already_set) {
        // SAFETY: called at the very start of `run`, before any thread is spawned.
        unsafe { env::set_var(DISABLE_DMABUF, "1") };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workaround_only_for_nvidia_on_wayland() {
        assert!(needs_dmabuf_workaround(true, true, false));
        assert!(!needs_dmabuf_workaround(false, true, false));
        assert!(!needs_dmabuf_workaround(true, false, false));
    }

    #[test]
    fn respects_the_users_own_setting() {
        assert!(!needs_dmabuf_workaround(true, true, true));
    }
}
