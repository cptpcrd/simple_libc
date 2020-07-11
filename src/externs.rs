// The libc crate is missing some functions. <sigh>
// Some of these just aren't there period; some aren't present for
// certain platforms that definitely have them.

extern "C" {
    pub fn getlogin_r(buf: *mut libc::c_char, bufsize: libc::size_t) -> libc::c_int;

    pub fn setreuid(ruid: libc::uid_t, euid: libc::uid_t) -> libc::c_int;
    pub fn setregid(rgid: libc::gid_t, egid: libc::gid_t) -> libc::c_int;

    pub fn gethostid() -> libc::c_long;

    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly"))]
    pub fn reboot(howto: libc::c_int) -> libc::c_int;

    #[cfg(target_os = "netbsd")]
    pub fn reboot(howto: libc::c_int, bootstr: *mut libc::c_char) -> libc::c_int;

    #[cfg(target_os = "netbsd")]
    pub fn pollts(
        fds: *mut libc::pollfd,
        nfds: libc::nfds_t,
        ts: *const libc::timespec,
        sigmask: *const libc::sigset_t,
    ) -> libc::c_int;
}

#[cfg(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "dragonfly",
))]
extern "C" {
    pub fn getresuid(
        ruid: *mut libc::uid_t,
        euid: *mut libc::uid_t,
        suid: *mut libc::uid_t,
    ) -> libc::c_int;
    pub fn setresuid(ruid: libc::uid_t, euid: libc::uid_t, suid: libc::uid_t) -> libc::c_int;

    pub fn getresgid(
        rgid: *mut libc::gid_t,
        egid: *mut libc::gid_t,
        sgid: *mut libc::gid_t,
    ) -> libc::c_int;
    pub fn setresgid(rgid: libc::gid_t, egid: libc::gid_t, sgid: libc::gid_t) -> libc::c_int;
}

#[cfg(any(
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
    target_os = "macos",
))]
extern "C" {
    pub fn issetugid() -> libc::c_int;
}

#[cfg(target_os = "linux")]
extern "C" {
    pub fn capget(
        hdrp: &mut crate::types::cap_user_header_t,
        datap: &mut crate::types::cap_user_data_t,
    ) -> libc::c_int;

    pub fn capset(
        hdrp: &mut crate::types::cap_user_header_t,
        datap: &crate::types::cap_user_data_t,
    ) -> libc::c_int;

    pub fn __libc_current_sigrtmin() -> libc::c_int;
    pub fn __libc_current_sigrtmax() -> libc::c_int;

    pub fn getauxval(t: libc::c_ulong) -> libc::c_ulong;
}
