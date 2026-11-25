use super::fs_result::from_host;

const EINTR: i64 = 4;

pub struct Doorbell {
    pub guest: i32,
    wake: i32,
    observe: i32,
}

impl Doorbell {
    pub fn new() -> Result<Self, i64> {
        let mut pair = [0i32; 2];
        if unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, pair.as_mut_ptr()) } < 0 {
            return Err(from_host());
        }
        let observe = unsafe { libc::fcntl(pair[0], libc::F_DUPFD_CLOEXEC, 3) };
        if observe < 0 {
            let failure = from_host();
            unsafe {
                libc::close(pair[0]);
                libc::close(pair[1]);
            }
            return Err(failure);
        }
        for fd in pair {
            unsafe {
                libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
                libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_NOSIGPIPE, &1i32 as *const i32 as *const libc::c_void, 4);
            }
        }
        unsafe { libc::fcntl(pair[1], libc::F_SETFD, libc::FD_CLOEXEC) };
        Ok(Self { guest: pair[0], wake: pair[1], observe })
    }

    pub fn ring(&self) {
        let byte = 1u8;
        unsafe { libc::send(self.wake, &byte as *const u8 as *const libc::c_void, 1, libc::MSG_DONTWAIT) };
    }

    pub fn drain(&self) {
        let mut sink = [0u8; 256];
        while unsafe { libc::recv(self.observe, sink.as_mut_ptr() as *mut libc::c_void, sink.len(), libc::MSG_DONTWAIT) } > 0 {}
    }

    pub fn wait_readable(&self) -> i64 {
        let mut request = libc::pollfd { fd: self.observe, events: libc::POLLIN, revents: 0 };
        if unsafe { libc::poll(&mut request, 1, -1) } < 0 {
            let failure = from_host();
            return if failure == -EINTR { failure } else { 0 };
        }
        0
    }
}

impl Drop for Doorbell {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.wake);
            libc::close(self.observe);
        }
    }
}
