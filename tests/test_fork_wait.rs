use simple_libc::process::fork;
use simple_libc::wait;

#[test]
fn test_fork_waitpid() {
    match fork().unwrap() {
        0 => std::process::exit(1),
        pid => {
            let (wpid, status) =
                wait::waitpid(wait::WaitpidSpec::Pid(pid), wait::WaitpidOptions::empty())
                    .unwrap()
                    .unwrap();

            assert_eq!(pid, wpid);

            assert_eq!(status, wait::ProcStatus::Exited(1));
        }
    }
}

#[test]
fn test_fork_wait4() {
    match fork().unwrap() {
        0 => std::process::exit(1),
        pid => {
            let (wpid, status, _rusage) =
                wait::wait4(wait::WaitpidSpec::Pid(pid), wait::WaitpidOptions::empty())
                    .unwrap()
                    .unwrap();

            assert_eq!(pid, wpid);

            assert_eq!(status, wait::ProcStatus::Exited(1));
        }
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "netbsd",
    target_os = "freebsd",
    target_os = "dragonfly",
))]
#[test]
fn test_fork_waitid() {
    match fork().unwrap() {
        0 => std::process::exit(1),
        pid => {
            let info = wait::waitid(wait::WaitidSpec::Pid(pid), wait::WaitidOptions::EXITED)
                .unwrap()
                .unwrap();

            assert_eq!(info.pid, pid);
            assert_eq!(info.uid, simple_libc::process::getuid());

            assert_eq!(info.status, wait::WaitidStatus::Exited(1));
        }
    }
}

#[cfg(any(
    target_os = "netbsd",
    target_os = "freebsd",
    target_os = "dragonfly",
))]
#[test]
fn test_fork_wait6() {
    match fork().unwrap() {
        0 => std::process::exit(1),
        pid => {
            let (status, info, _self_rusage, _child_rusage) =
                wait::wait6(wait::WaitidSpec::Pid(pid), wait::WaitidOptions::EXITED)
                    .unwrap()
                    .unwrap();

            assert_eq!(info.pid, pid);
            assert_eq!(info.uid, simple_libc::process::getuid());
            assert_eq!(info.status, wait::WaitidStatus::Exited(1));

            assert_eq!(status, wait::ProcStatus::Exited(1));
        }
    }
}
