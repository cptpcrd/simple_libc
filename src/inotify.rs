use std::ffi;
use std::io;
use std::os::unix::prelude::*;
use std::path::Path;

use bitflags::bitflags;

use crate::Int;
use crate::constants;

bitflags! {
    pub struct Events: u32 {
        const OPEN = libc::IN_OPEN;
        const ATTRIB = libc::IN_ATTRIB;
        const ACCESS = libc::IN_ACCESS;
        const MODIFY = libc::IN_MODIFY;

        const CLOSE_WRITE = libc::IN_CLOSE_WRITE;
        const CLOSE_NOWRITE = libc::IN_CLOSE_NOWRITE;

        const CREATE = libc::IN_CREATE;
        const DELETE = libc::IN_DELETE;

        const DELETE_SELF = libc::IN_DELETE_SELF;
        const MOVE_SELF = libc::IN_MOVE_SELF;

        const MOVED_FROM = libc::IN_MOVED_FROM;
        const MOVED_TO = libc::IN_MOVED_TO;

        const MOVE = libc::IN_MOVE;
        const CLOSE = libc::IN_CLOSE;
        const ALL_EVENTS = libc::IN_ALL_EVENTS;
    }
}

bitflags! {
    pub struct WatchFlags: u32 {
        const DONT_FOLLOW = libc::IN_DONT_FOLLOW;
        const ONESHOT = libc::IN_ONESHOT;
        const ONLYDIR = libc::IN_ONLYDIR;

        const EXCL_UNLINK = constants::IN_EXCL_UNLINK;
    }
}

bitflags! {
    pub struct EventFlags: u32 {
        const IGNORED = libc::IN_IGNORED;
        const ISDIR = libc::IN_ISDIR;
        const Q_OVERFLOW = libc::IN_Q_OVERFLOW;
        const UNMOUNT = libc::IN_UNMOUNT;
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub watch: Watch,
    pub events: Events,
    pub flags: EventFlags,
    pub cookie: u32,
    pub name: ffi::OsString,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Watch {
    wd: i32,
}

#[derive(Debug)]
pub struct Inotify {
    fd: i32,
}

const RAW_EVENT_SIZE: usize = std::mem::size_of::<libc::inotify_event>();

impl Inotify {
    /// Construct a new inotify file descriptor with the given options.
    pub fn new(nonblock: bool) -> io::Result<Self> {
        let mut flags = libc::IN_CLOEXEC;
        if nonblock {
            flags |= libc::IN_NONBLOCK;
        }

        let fd = crate::error::convert_neg_ret(unsafe { libc::inotify_init1(flags) })?;

        Ok(Self { fd })
    }

    fn add_watch_impl<P: AsRef<Path>>(&mut self, path: P, flags: u32) -> io::Result<Watch> {
        let c_path = ffi::CString::new(path.as_ref().as_os_str().as_bytes())?;

        let wd = crate::error::convert_neg_ret(unsafe {
            libc::inotify_add_watch(self.fd, c_path.as_ptr(), flags)
        })?;

        Ok(Watch { wd })
    }

    /// Add a new watch (or modify an existing watch) for the given file.
    #[inline]
    pub fn add_watch<P: AsRef<Path>>(
        &mut self,
        path: P,
        events: Events,
        flags: WatchFlags,
    ) -> io::Result<Watch> {
        self.add_watch_impl(path, events.bits | flags.bits)
    }

    /// Add a new watch for the given file, or extend the watch mask if the watch already exists.
    #[inline]
    pub fn extend_watch<P: AsRef<Path>>(
        &mut self,
        path: P,
        events: Events,
        flags: WatchFlags,
    ) -> io::Result<Watch> {
        self.add_watch_impl(path, events.bits | flags.bits | constants::IN_MASK_ADD)
    }

    /// Add a new watch for the given file, failing with an error if the event already exists
    #[inline]
    pub fn create_watch<P: AsRef<Path>>(
        &mut self,
        path: P,
        events: Events,
        flags: WatchFlags,
    ) -> io::Result<Watch> {
        self.add_watch_impl(path, events.bits | flags.bits | constants::IN_MASK_CREATE)
    }

    /// Remove a previously added watch.
    pub fn rm_watch(&mut self, watch: Watch) -> io::Result<()> {
        crate::error::convert_nzero_ret(unsafe { libc::inotify_rm_watch(self.fd, watch.wd) })
    }

    /// Read a list of events from the inotify file descriptor, or return an
    /// empty vector if no events are pending.
    pub fn read_nowait(&mut self) -> io::Result<Vec<Event>> {
        // See how much data is ready for reading
        let nbytes = crate::ioctl::get_readbuf_length()?;

        // No data? Return an empty vector.
        if nbytes == 0 {
            return Ok(Vec::new());
        }

        // Prepare a buffer
        let mut buf: Vec<u8> = Vec::new();
        buf.resize(nbytes, 0);

        // Read the data
        let nbytes: isize = crate::error::convert_neg_ret(unsafe {
            libc::read(
                self.fd,
                buf.as_mut_ptr() as *mut libc::c_void,
                nbytes,
            )
        })?;

        // Trim down if we read less data
        buf.resize(nbytes, 0);

        Ok(Self::parse_multi(&buf))
    }

    /// Read a list of events from the inotify file descriptor, waiting for
    /// an event to occur if none are pending.
    ///
    /// Note: The current implementation of this function may not read *all* of the
    /// events from the the inotify file descriptor. If that is necessary for whatever
    /// reason, it is recommended to immediately follow a call to `read_wait()` with a
    /// call to `read_nowait()` and combine the two resulting vectors.
    pub fn read_wait(&mut self) -> io::Result<Vec<Event>> {
        let mut buf: Vec<u8> = Vec::new();

        // In the extremely rare event that this isn't enough to read a single event, we'll expand it later.
        // In practice, it'll usually be enough to read at least a few dozen events.
        buf.resize(4096, 0);

        let mut i = 0;
        loop {
            match crate::error::convert_neg_ret(unsafe {
                libc::read(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len())
            }) {
                Ok(nbytes) => {
                    // Trim down to size
                    buf.resize(nbytes as usize, 0);

                    // And parse the events
                    return Ok(Self::parse_multi(&buf));
                }
                Err(e) => {
                    if i < 10 && crate::error::is_einval(&e) {
                        buf.resize(buf.len() * 2, 0);
                    } else {
                        return Err(e);
                    }
                }
            }

            i += 1;
        }
    }

    fn parse_multi(data: &[u8]) -> Vec<Event> {
        let mut events: Vec<Event> = Vec::new();
        let mut offset: usize = 0;
        while offset < data.len() {
            let (event, inc) = Self::parse_one(&data[offset..]);
            events.push(event);
            offset += inc;
        }

        events
    }

    fn parse_one(data: &[u8]) -> (Event, usize) {
        debug_assert!(data.len() >= RAW_EVENT_SIZE);

        // Extract the raw event
        #[allow(clippy::transmute_ptr_to_ref)]
        let raw_event =
            unsafe { std::mem::transmute::<*const u8, &libc::inotify_event>(data.as_ptr()) };

        // Extract the name.
        //
        // We skip over the initial structure,
        // limit our traversal to the length we were given,
        // stop at the first null byte,
        // clone the elements so we don't have to mess with references,
        // collect into a vector,
        // and convert that into an OsString.
        let name = ffi::OsString::from_vec(
            data.iter()
                .skip(RAW_EVENT_SIZE)
                .take(raw_event.len as usize)
                .take_while(|x| **x != 0)
                .cloned()
                .collect(),
        );

        // Now return the events and the number of bytes we consumed.
        (
            Event {
                watch: Watch { wd: raw_event.wd },
                events: Events::from_bits_truncate(raw_event.mask),
                flags: EventFlags::from_bits_truncate(raw_event.mask),
                cookie: raw_event.cookie,
                name,
            },
            RAW_EVENT_SIZE + raw_event.len as usize,
        )
    }
}

impl AsRawFd for Inotify {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
}

impl Drop for Inotify {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}
