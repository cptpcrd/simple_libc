use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use bitflags::bitflags;
use libc;

bitflags! {
    pub struct ExtraUnshareFlags: i32 {
        const FILES = libc::CLONE_FILES;
        const FS = libc::CLONE_FS;
        const SYSVSEM = libc::CLONE_SYSVSEM;
    }
}

bitflags! {
    pub struct NamespaceTypes: i32 {
        const NEWCGROUP = libc::CLONE_NEWCGROUP;
        const NEWIPC = libc::CLONE_NEWIPC;
        const NEWNET = libc::CLONE_NEWNET;
        const NEWNS = libc::CLONE_NEWNS;
        const NEWPID = libc::CLONE_NEWPID;
        const NEWUSER = libc::CLONE_NEWUSER;
        const NEWUTS = libc::CLONE_NEWUTS;
    }
}

pub fn unshare(nstypes: NamespaceTypes, extra_flags: ExtraUnshareFlags) -> io::Result<()> {
    super::super::error::convert_nzero(
        unsafe { libc::unshare(nstypes.bits | extra_flags.bits) },
        (),
    )
}

pub fn setns_raw(fd: i32, nstype: NamespaceTypes) -> io::Result<()> {
    super::super::error::convert_nzero(unsafe { libc::setns(fd, nstype.bits) }, ())
}

pub fn setns(f: &File, nstype: NamespaceTypes) -> io::Result<()> {
    setns_raw(f.as_raw_fd(), nstype)
}

pub fn join_proc_namespaces<P: AsRef<Path>>(
    proc_pid_dir: P,
    mut nstypes: NamespaceTypes,
) -> io::Result<()> {
    let proc_ns_dir = proc_pid_dir.as_ref().join("ns");

    if nstypes.contains(NamespaceTypes::NEWUSER) {
        // Switching user namespaces (usually) gives us CAP_SYS_ADMIN in the new namespace, which
        // lets us perform more operations.
        let file = File::open(&proc_ns_dir.join("user"))?;
        setns(&file, NamespaceTypes::NEWUSER)?;

        nstypes.remove(NamespaceTypes::NEWUSER);
    }

    for entry in proc_ns_dir.read_dir()? {
        let entry = entry?;

        if let Some(name) = entry.file_name().to_str() {
            // Figure out which namespace it is
            let entry_nstype = match name {
                "cgroup" => NamespaceTypes::NEWCGROUP,
                "ipc" => NamespaceTypes::NEWIPC,
                "net" => NamespaceTypes::NEWNET,
                "mnt" => NamespaceTypes::NEWNS,
                "pid" => NamespaceTypes::NEWPID,
                "uts" => NamespaceTypes::NEWUTS,
                _ => NamespaceTypes::empty(),
            };

            if !entry_nstype.is_empty() && nstypes.contains(entry_nstype) {
                let file = File::open(entry.path())?;
                setns(&file, entry_nstype)?;

                // Now remove it from the bitmask so we know we've joined it
                nstypes.remove(entry_nstype);
            }
        }
    }

    if !nstypes.is_empty() {
        // Extra flags were passed that we didn't recognize
        return Err(io::Error::from_raw_os_error(libc::EINVAL));
    }

    Ok(())
}
