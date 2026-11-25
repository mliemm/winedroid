use super::bionic::linux_abi::{at, open as o};
use super::io_test_support::{call, close, guest_errno, open, read_some, write_all, IoGuest, Text};
use super::test_deadline::bounded_test;

bounded_test! {
    fn bionic_open_write_read_seek_and_stat_use_linux_layouts_inside_the_guest_prefix() {
        // cite: bionic stat.h#__STAT64_BODY
        let guest = IoGuest::new("fs-basic");
        let path = guest.guest("hello.txt");
        let fd = open(&path, o::RDWR | o::CREAT | o::TRUNC, 0o644);
        assert!(fd >= 3, "open failed: errno {}", guest_errno());
        assert_eq!(write_all(fd, b"hello, world"), 12);
        assert_eq!(call("lseek").i32(fd).int(7).i32(0).invoke().unwrap().u64(), 7);
        assert_eq!(read_some(fd, 16), b"world");

        let mut stat = [0u8; 128];
        assert_eq!(call("fstat").i32(fd).ptr(stat.as_mut_ptr() as usize).invoke().unwrap().i32(), 0);
        assert_eq!(i64::from_le_bytes(stat[48..56].try_into().unwrap()), 12, "st_size sits at offset 48");
        assert_eq!(u32::from_le_bytes(stat[16..20].try_into().unwrap()) & 0o170000, 0o100000, "st_mode sits at offset 16");
        assert_eq!(u32::from_le_bytes(stat[24..28].try_into().unwrap()), 10000, "st_uid is the app uid");
        assert_eq!(close(fd), 0);

        assert_eq!(std::fs::read(guest.host("hello.txt")).unwrap(), b"hello, world");
        let text = Text::new(&path);
        let mut by_path = [0u8; 128];
        assert_eq!(call("stat").ptr(text.ptr()).ptr(by_path.as_mut_ptr() as usize).invoke().unwrap().i32(), 0);
        assert_eq!(by_path[48..56], stat[48..56]);
    }
}

bounded_test! {
    fn bionic_close_of_a_descriptor_the_guest_does_not_own_fails_with_ebadf_and_keeps_it_open() {
        // cite: bionic fdsan.cpp#close
        let _guest = IoGuest::new("fs-owned");
        let mut ends = [0i32; 2];
        assert_eq!(unsafe { libc::pipe(ends.as_mut_ptr()) }, 0);

        assert_eq!(close(ends[0]), -1);
        assert_eq!(guest_errno(), 9);
        assert_eq!(unsafe { libc::fcntl(ends[0], libc::F_GETFD) } >= 0, true, "the host descriptor stays open");
        unsafe {
            libc::close(ends[0]);
            libc::close(ends[1]);
        }
    }
}

bounded_test! {
    fn bionic_open_translates_linux_flags_to_the_host_and_f_getfl_reads_them_back() {
        // cite: bionic fcntl.h#O_CREAT, O_EXCL, O_NONBLOCK, O_APPEND, O_CLOEXEC
        let guest = IoGuest::new("fs-flags");
        let path = guest.guest("flags.bin");
        let fd = open(&path, o::WRONLY | o::CREAT | o::EXCL | o::APPEND | o::NONBLOCK | o::CLOEXEC, 0o600);
        assert!(fd >= 3, "errno {}", guest_errno());
        let flags = call("fcntl").i32(fd).i32(3).int(0).invoke().unwrap().i32();
        assert_eq!(flags & o::ACCMODE, o::WRONLY);
        assert_ne!(flags & o::APPEND, 0);
        assert_ne!(flags & o::NONBLOCK, 0);
        assert_ne!(flags & o::LARGEFILE, 0);
        assert_eq!(flags & (o::CREAT | o::EXCL), 0, "creation flags are not reported");
        assert_eq!(call("fcntl").i32(fd).i32(1).int(0).invoke().unwrap().i32() & 1, 1, "FD_CLOEXEC is set");
        assert_eq!(close(fd), 0);
        assert_eq!(open(&path, o::WRONLY | o::CREAT | o::EXCL, 0o600), -1);
        assert_eq!(guest_errno(), 17, "O_EXCL on an existing file is EEXIST");
    }
}

bounded_test! {
    fn bionic_directory_listing_returns_280_byte_dirents_with_the_name_at_offset_19() {
        // cite: bionic dirent.h#__DIRENT64_BODY
        let guest = IoGuest::new("fs-dirent");
        std::fs::write(guest.host("alpha"), b"1").unwrap();
        std::fs::create_dir(guest.host("beta")).unwrap();
        let path = Text::new(&guest.guest(""));
        let dir = call("opendir").ptr(path.ptr()).invoke().unwrap().u64() as usize;
        assert_ne!(dir, 0);
        let mut seen = Vec::new();
        loop {
            let entry = call("readdir").ptr(dir).invoke().unwrap().u64() as usize;
            if entry == 0 {
                break;
            }
            let name = unsafe { std::ffi::CStr::from_ptr((entry + 19) as *const _) }.to_string_lossy().into_owned();
            let kind = unsafe { *((entry + 18) as *const u8) };
            let reclen = unsafe { ((entry + 16) as *const u16).read_unaligned() };
            assert_eq!(reclen as usize % 8, 0);
            seen.push((name, kind));
        }
        assert_eq!(call("closedir").ptr(dir).invoke().unwrap().i32(), 0);
        seen.sort();
        assert_eq!(seen, vec![(".".to_string(), 4), ("..".to_string(), 4), ("alpha".to_string(), 8), ("beta".to_string(), 4)]);
    }
}

bounded_test! {
    fn bionic_mkdir_rename_unlink_and_rmdir_work_through_guest_paths_and_apply_the_umask() {
        // cite: bionic dirent.cpp#opendir
        let guest = IoGuest::new("fs-ops");
        call("umask").u32(0o077).invoke().unwrap();
        let dir = Text::new(&guest.guest("made"));
        assert_eq!(call("mkdir").ptr(dir.ptr()).u32(0o777).invoke().unwrap().i32(), 0);
        let mode = std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(guest.host("made")).unwrap().permissions()) & 0o777;
        assert_eq!(mode, 0o700, "the guest umask 077 removes group and other bits");
        assert_eq!(call("mkdir").ptr(dir.ptr()).u32(0o777).invoke().unwrap().i32(), -1);
        assert_eq!(guest_errno(), 17);

        let file = guest.guest("made/a");
        let fd = open(&file, o::WRONLY | o::CREAT, 0o600);
        close(fd);
        let moved = Text::new(&guest.guest("made/b"));
        let old = Text::new(&file);
        assert_eq!(call("rename").ptr(old.ptr()).ptr(moved.ptr()).invoke().unwrap().i32(), 0);
        assert!(guest.host("made/b").exists() && !guest.host("made/a").exists());
        assert_eq!(call("rmdir").ptr(dir.ptr()).invoke().unwrap().i32(), -1);
        assert_eq!(guest_errno(), 39, "a non empty directory is ENOTEMPTY");
        assert_eq!(call("unlink").ptr(moved.ptr()).invoke().unwrap().i32(), 0);
        assert_eq!(call("rmdir").ptr(dir.ptr()).invoke().unwrap().i32(), 0);
        assert_eq!(call("unlink").ptr(dir.ptr()).invoke().unwrap().i32(), -1);
        assert_eq!(guest_errno(), 2);
    }
}

bounded_test! {
    fn bionic_paths_outside_the_guest_namespace_never_reach_the_host() {
        // cite: frameworks-base StrictMode.java#onCredentialProtectedPathAccess
        let guest = IoGuest::new("fs-escape");
        for path in ["/etc/hosts", "/Users", "/data/user/0/com.example.io/../../../../etc/passwd", "/data/user/0/com.other.app/files", "/tmp"] {
            assert_eq!(open(path, o::RDONLY, 0), -1, "{path}");
            assert!([2, 13].contains(&guest_errno()), "{path}: errno {}", guest_errno());
        }
        let alias = Text::new("/data/data/com.example.io");
        let mut stat = [0u8; 128];
        assert_eq!(call("stat").ptr(alias.ptr()).ptr(stat.as_mut_ptr() as usize).invoke().unwrap().i32(), 0, "/data/data is an alias of the credential protected root");
        let relative = Text::new("relative.txt");
        assert_eq!(call("chdir").ptr(Text::new(&guest.guest("")).ptr()).invoke().unwrap().i32(), 0);
        let fd = call("open").ptr(relative.ptr()).i32(o::WRONLY | o::CREAT).u32(0o600).invoke().unwrap().i32();
        assert!(fd >= 3);
        close(fd);
        assert!(guest.host("relative.txt").exists(), "relative paths resolve against the guest cwd");
        let _ = at::FDCWD;
    }
}

bounded_test! {
    fn bionic_pipe_and_dup_share_the_open_description_and_close_range_closes_owned_descriptors_only() {
        // cite: bionic pipe.cpp#pipe2
        let _guest = IoGuest::new("fs-pipe");
        let mut ends = [0i32; 2];
        assert_eq!(call("pipe2").ptr(ends.as_mut_ptr() as usize).i32(o::CLOEXEC).invoke().unwrap().i32(), 0);
        assert_eq!(write_all(ends[1], b"ping"), 4);
        let copy = call("dup").i32(ends[0]).invoke().unwrap().i32();
        assert!(copy > 2);
        assert_eq!(read_some(copy, 8), b"ping");
        assert_eq!(close(copy), 0);
        assert_eq!(call("pipe2").ptr(ends.as_mut_ptr() as usize).i32(o::DIRECT).invoke().unwrap().i32(), -1);
        assert_eq!(guest_errno(), 22, "packet mode pipes are refused");
    }
}
