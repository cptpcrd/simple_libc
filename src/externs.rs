use cfg_if::cfg_if;

// The libc crate is missing some functions. <sigh>
// Some of these just aren't there period; some aren't present for
// certain platforms that definitely have them.

extern "C" {
    pub fn getlogin_r(buf: *mut libc::c_char, bufsize: libc::size_t) -> libc::c_int;

    pub fn setreuid(ruid: libc::uid_t, euid: libc::uid_t) -> libc::c_int;
    pub fn setregid(rgid: libc::gid_t, egid: libc::gid_t) -> libc::c_int;

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

cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd", target_os = "dragonfly"))] {
        extern "C" {
            pub fn getresuid(ruid: *mut libc::uid_t, euid: *mut libc::uid_t, suid: *mut libc::uid_t) -> libc::c_int;
            pub fn setresuid(ruid: libc::uid_t, euid: libc::uid_t, suid: libc::uid_t) -> libc::c_int;

            pub fn getresgid(rgid: *mut libc::gid_t, egid: *mut libc::gid_t, sgid: *mut libc::gid_t) -> libc::c_int;
            pub fn setresgid(rgid: libc::gid_t, egid: libc::gid_t, sgid: libc::gid_t) -> libc::c_int;
        }
    }
}
