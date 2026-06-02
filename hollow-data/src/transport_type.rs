// SPDX-License-Identifier: GPL-3.0-only

//!
//! tokio/mio already uses the OS-native reactor (epoll on Linux, kqueue on macOS,
//! IOCP on Windows). We reproduce Hollow's availability check + NIO fallback +
//! logging semantics; the configured type does not change the underlying reactor,
//! except IO_URING which would require the optional `io-uring` build feature.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportType {
    Nio,
    Epoll,
    IoUring,
    Kqueue,
}

impl TransportType {
    pub fn name(self) -> &'static str {
        match self {
            TransportType::Nio => "NIO",
            TransportType::Epoll => "EPOLL",
            TransportType::IoUring => "IO_URING",
            TransportType::Kqueue => "KQUEUE",
        }
    }

    pub fn from_name(name: &str) -> Option<TransportType> {
        Some(match name.to_ascii_uppercase().as_str() {
            "NIO" => TransportType::Nio,
            "EPOLL" => TransportType::Epoll,
            "IO_URING" => TransportType::IoUring,
            "KQUEUE" => TransportType::Kqueue,
            _ => return None,
        })
    }

    /// Whether this transport is usable on the current platform (mirrors isAvailable()).
    pub fn is_available(self) -> bool {
        match self {
            TransportType::Nio => true,
            TransportType::Epoll => cfg!(target_os = "linux"),
            TransportType::IoUring => cfg!(target_os = "linux") && cfg!(feature = "io-uring"),
            TransportType::Kqueue => cfg!(any(
                target_os = "macos",
                target_os = "ios",
                target_os = "freebsd",
                target_os = "netbsd",
                target_os = "openbsd",
                target_os = "dragonfly"
            )),
        }
    }
}
