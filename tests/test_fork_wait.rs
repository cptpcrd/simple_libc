use simple_libc::process::fork;
use simple_libc::wait;

#[test]
fn test_fork() {
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