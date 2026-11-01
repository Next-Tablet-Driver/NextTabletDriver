//! What the machine is: its memory, its distribution and its name, read from `/proc` and `/etc`.

use crate::SystemInfo;

/// The installed memory, the distribution name and the host name.
pub(crate) fn system_info() -> SystemInfo {
    SystemInfo {
        total_memory_bytes: std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|text| total_memory_bytes(&text)),
        distribution: std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|text| distribution(&text)),
        hostname: std::env::var("HOSTNAME").ok().or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|name| name.trim().to_string())
        }),
    }
}

/// `MemTotal` of `/proc/meminfo`, in bytes (the file counts kibibytes).
fn total_memory_bytes(meminfo: &str) -> Option<u64> {
    let line = meminfo.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kibibytes = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    kibibytes.checked_mul(1024)
}

/// `PRETTY_NAME` of `/etc/os-release`.
fn distribution(os_release: &str) -> Option<String> {
    os_release
        .lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|name| name.trim_matches('"').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_memory_is_read_in_bytes() {
        assert_eq!(
            total_memory_bytes("MemTotal:       16384 kB\nMemFree: 1 kB\n"),
            Some(16_777_216)
        );
        assert_eq!(total_memory_bytes("MemFree: 1 kB\n"), None);
    }

    #[test]
    fn the_distribution_loses_its_quotes() {
        assert_eq!(
            distribution("NAME=x\nPRETTY_NAME=\"Debian GNU/Linux 12\"\n").as_deref(),
            Some("Debian GNU/Linux 12")
        );
        assert_eq!(distribution("NAME=x\n"), None);
    }
}
