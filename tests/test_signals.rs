use std::io;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "linux")] {
        use simple_libc::sigmask;
        use simple_libc::signalfd;
        use simple_libc::process::{getpid, gettid, getuid};
        use simple_libc::signal::{Sigset, SIGUSR1, SIGUSR2};
        use simple_libc::{tgkill, Int, PidT, UidT};

        #[test]
        fn test_signalfd() {
            // Sanity check
            assert_eq!(
                std::mem::size_of::<signalfd::Siginfo>(),
                std::mem::size_of::<libc::signalfd_siginfo>(),
            );

            // Block the signals we're going to send
            let orig_mask = sigmask::getmask().unwrap();
            let mut new_mask = Sigset::empty();
            new_mask.add(SIGUSR1).unwrap();
            new_mask.add(SIGUSR2).unwrap();
            sigmask::block(&new_mask).unwrap();

            // Create a signalfd
            let sigfd =
                signalfd::SignalFd::new(&Sigset::full(), libc::SFD_CLOEXEC | libc::SFD_NONBLOCK).unwrap();

            let mut sigs = [signalfd::Siginfo::default(); 3];

            // Make sure nothing's pending
            assert_eq!(
                sigfd.read(&mut sigs).unwrap_err().kind(),
                io::ErrorKind::WouldBlock,
            );
            assert_eq!(
                sigfd.read_one().unwrap_err().kind(),
                io::ErrorKind::WouldBlock,
            );

            // Send a signal to ourselves
            tgkill(getpid(), gettid(), SIGUSR1).unwrap();
            assert_eq!(sigfd.read(&mut sigs).unwrap(), 1);
            assert_eq!(sigs[0].sig as Int, SIGUSR1);
            assert_eq!(sigs[0].pid as PidT, getpid());
            assert_eq!(sigs[0].uid as UidT, getuid());

            // Send another signal
            tgkill(getpid(), gettid(), SIGUSR1).unwrap();
            let siginfo = sigfd.read_one().unwrap();
            assert_eq!(siginfo.sig as Int, SIGUSR1);
            assert_eq!(siginfo.pid as PidT, getpid());
            assert_eq!(siginfo.uid as UidT, getuid());

            // Send two signals
            tgkill(getpid(), gettid(), SIGUSR1).unwrap();
            tgkill(getpid(), gettid(), SIGUSR2).unwrap();
            assert_eq!(sigfd.read(&mut sigs).unwrap(), 2);
            assert_eq!(sigs[0].sig as Int, SIGUSR1);
            assert_eq!(sigs[0].pid as PidT, getpid());
            assert_eq!(sigs[0].uid as UidT, getuid());
            assert_eq!(sigs[1].sig as Int, SIGUSR2);
            assert_eq!(sigs[1].pid as PidT, getpid());
            assert_eq!(sigs[1].uid as UidT, getuid());

            // Restore our signal mask
            sigmask::setmask(&orig_mask).unwrap();
        }
    }
}
