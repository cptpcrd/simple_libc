#[cfg(any(target_os = "linux", target_os = "openbsd"))]
pub mod ucred;
#[cfg(any(target_os = "linux", target_os = "openbsd"))]
pub use ucred::{get_ucred, get_ucred_raw, Ucred};
