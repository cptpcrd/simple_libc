#[cfg(any(target_os = "linux", target_os = "openbsd"))]
pub mod ucred;
#[cfg(any(target_os = "linux", target_os = "openbsd"))]
pub use ucred::{get_ucred, get_ucred_raw, Ucred};

#[cfg(any(target_os = "linux"))]
mod abstract_unix;
#[cfg(any(target_os = "linux"))]
pub use abstract_unix::{unix_stream_abstract_bind, unix_stream_abstract_connect};
