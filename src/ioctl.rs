use std::io;

use crate::error;
use crate::Int;

macro_rules! ioctl_raw {
    ($fd:expr, $cmd:expr$(, $args:expr)*) => {
        error::convert_ret(libc::ioctl($fd, $cmd$(, $args)*))
    };
}

pub fn get_readbuf_length(fd: Int) -> io::Result<usize> {
    let mut nbytes: Int = 0;

    unsafe {
        ioctl_raw!(fd, libc::FIONREAD, &mut nbytes)?;
    }

    Ok(if nbytes > 0 { nbytes as usize } else { 0 })
}

#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use std::os::unix::prelude::*;

    use super::*;

    #[test]
    fn test_get_readbuf_length() {
        let (mut r, mut w) = crate::pipe().unwrap();

        w.write_all(&[1, 2]).unwrap();
        w.flush().unwrap();
        drop(w);

        assert_eq!(get_readbuf_length(r.as_raw_fd()).unwrap(), 2);

        let mut buf = Vec::new();
        assert_eq!(r.read_to_end(&mut buf).unwrap(), 2);
        assert_eq!(buf, vec![1, 2]);
    }
}
