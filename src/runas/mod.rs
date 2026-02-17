mod shared;
pub use self::shared::{Child, Command};

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) use self::windows::{runas_spawn, ChildInner};

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod macos;
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) use self::macos::{runas_spawn, ChildInner};

#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
mod unix;
#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
pub(crate) use self::unix::{runas_spawn, ChildInner};
