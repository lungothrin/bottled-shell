
pub fn get_preserved_env() -> Vec<String> {
    use std::collections::BTreeSet;
    use literal::{set,SetLiteral};

    let preserved_env: BTreeSet<String> = set!{
        "WSLENV",
        "WSL_INTEROP",
        "WSL_DISTRO_NAME",
        "WSL_NAME",
        "WT_SESSION",
        "WT_PROFILE_ID",
        "PULSE_SERVER",
        "WAYLAND_DISPLAY",
        "BOTTLED_SHELL_LOG",
    };

    std::env::vars()
        .into_iter()
        .filter_map(|(k, v)| {
            if preserved_env.contains(&k) {
                return Some(format!("{}={}", k, v))
            }
            for p in k.split('_') {
                if p == "WT" {
                    return Some(format!("{}={}", k, v))
                } else if p.starts_with("WSL") {
                    return Some(format!("{}={}", k, v))
                } else if p.ends_with("WSL") || p.ends_with("WSL2") {
                    return Some(format!("{}={}", k, v))
                }
            }
            None
        })
        .collect::<Vec<_>>()
}