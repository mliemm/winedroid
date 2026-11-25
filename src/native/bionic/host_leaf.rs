use std::ffi::CStr;
use std::os::fd::AsRawFd;

use super::fs_result::from_host;
use super::guest_paths::HostRef;

const CURRENT: &CStr = c".";

impl HostRef {
    pub fn dir_fd(&self) -> i32 {
        self.dir.as_fd().as_raw_fd()
    }

    pub fn name(&self) -> &CStr {
        self.leaf.as_deref().unwrap_or(CURRENT)
    }

    pub fn stat(&self) -> Result<libc::stat, i32> {
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let status = unsafe { libc::fstatat(self.dir_fd(), self.name().as_ptr(), &mut stat, libc::AT_SYMLINK_NOFOLLOW) };
        if status < 0 {
            return Err((-from_host()) as i32);
        }
        Ok(stat)
    }

    pub fn link_target(&self) -> Option<String> {
        self.leaf.as_ref()?;
        let stat = self.stat().ok()?;
        if u32::from(stat.st_mode) & super::stat_convert::S_IFMT != super::stat_convert::S_IFLNK {
            return None;
        }
        let mut buffer = vec![0u8; 4096];
        let length = unsafe { libc::readlinkat(self.dir_fd(), self.name().as_ptr(), buffer.as_mut_ptr() as *mut libc::c_char, buffer.len()) };
        (length >= 0).then(|| String::from_utf8_lossy(&buffer[..length as usize]).into_owned())
    }

    pub fn open(&self, flags: i32, mode: u32) -> i64 {
        let fd = unsafe { libc::openat(self.dir_fd(), self.name().as_ptr(), flags, mode as libc::c_uint) };
        if fd < 0 {
            from_host()
        } else {
            i64::from(fd)
        }
    }
}
