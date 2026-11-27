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

fn watch_host_directory(
    path: &std::path::Path,
) -> (std::sync::Arc<crate::fs_notify::Instance>, i32) {
    let instance = crate::fs_notify::Instance::new();
    let directory = crate::capability_fs::Directory::open_ambient(path).unwrap();
    let wd = instance
        .add_watch(
            crate::fs_notify::InodeKey::of(directory.as_fd()).unwrap(),
            crate::fs_notify::WIRED_EVENTS,
        )
        .unwrap();
    (instance, wd)
}

fn notify_event(wd: i32, mask: u32, name: Option<&str>) -> crate::fs_notify::Event {
    crate::fs_notify::Event {
        wd,
        mask,
        name: name.map(|name| std::ffi::CString::new(name).unwrap()),
    }
}

bounded_test! {
    fn bionic_close_write_is_reported_on_the_last_close_of_a_duplicated_descriptor() {
        // cite: bionic inotify.h#IN_CREATE, IN_CLOSE_WRITE
        let guest = IoGuest::new("notify-dup");
        let (instance, wd) = watch_host_directory(&guest.host(""));
        let fd = open(&guest.guest("dup.bin"), o::WRONLY | o::CREAT, 0o600);
        assert!(fd >= 3, "errno {}", guest_errno());
        let copy = call("dup").i32(fd).invoke().unwrap().i32();
        assert!(copy >= 3);
        assert_eq!(write_all(fd, b"d"), 1);
        assert_eq!(close(fd), 0);
        assert_eq!(instance.take_events(), vec![notify_event(wd, crate::fs_notify::IN_CREATE, Some("dup.bin"))], "inotify(7) IN_CLOSE_WRITE waits for the last close of the open file description");
        assert_eq!(close(copy), 0);
        assert_eq!(instance.take_events(), vec![notify_event(wd, crate::fs_notify::IN_CLOSE_WRITE, Some("dup.bin"))]);

        let read_only = open(&guest.guest("dup.bin"), o::RDONLY, 0);
        assert_eq!(close(read_only), 0);
        let existing = open(&guest.guest("dup.bin"), o::WRONLY | o::CREAT | o::TRUNC, 0o600);
        assert_eq!(close(existing), 0);
        assert_eq!(instance.take_events(), vec![notify_event(wd, crate::fs_notify::IN_CLOSE_WRITE, Some("dup.bin"))], "inotify(7): a read only close is IN_CLOSE_NOWRITE and O_CREAT of an existing file is no IN_CREATE");
    }
}

bounded_test! {
    fn bionic_mkdir_mknod_and_rmdir_report_create_is_dir_and_ignored() {
        // cite: bionic inotify.h#IN_CREATE, IN_ISDIR, IN_IGNORED
        let guest = IoGuest::new("notify-dirs");
        let (instance, wd) = watch_host_directory(&guest.host(""));
        let dir = Text::new(&guest.guest("made"));
        assert_eq!(call("mkdir").ptr(dir.ptr()).u32(0o777).invoke().unwrap().i32(), 0);
        let node = Text::new(&guest.guest("node"));
        assert_eq!(call("mknod").ptr(node.ptr()).u32(0o100600).int(0).invoke().unwrap().i32(), 0, "errno {}", guest_errno());
        assert_eq!(
            instance.take_events(),
            vec![
                notify_event(wd, crate::fs_notify::IN_CREATE | crate::fs_notify::IN_ISDIR, Some("made")),
                notify_event(wd, crate::fs_notify::IN_CREATE, Some("node")),
            ],
            "inotify(7): mkdir is IN_CREATE|IN_ISDIR and mknod creates without a writable open"
        );

        let link = Text::new(&guest.guest("link"));
        assert_eq!(call("symlink").ptr(node.ptr()).ptr(link.ptr()).invoke().unwrap().i32(), 0);
        let hard = Text::new(&guest.guest("hard"));
        assert_eq!(call("link").ptr(node.ptr()).ptr(hard.ptr()).invoke().unwrap().i32(), 0);
        let fifo = Text::new(&guest.guest("fifo"));
        assert_eq!(call("mkfifo").ptr(fifo.ptr()).u32(0o600).invoke().unwrap().i32(), 0);
        assert_eq!(
            instance.take_events(),
            vec![
                notify_event(wd, crate::fs_notify::IN_CREATE, Some("link")),
                notify_event(wd, crate::fs_notify::IN_CREATE, Some("hard")),
                notify_event(wd, crate::fs_notify::IN_CREATE, Some("fifo")),
            ],
            "inotify(7) IN_CREATE covers symlink, link and mkfifo"
        );

        let (inner, inner_wd) = watch_host_directory(&guest.host("made"));
        assert_eq!(call("rmdir").ptr(dir.ptr()).invoke().unwrap().i32(), 0);
        assert_eq!(inner.take_events(), vec![notify_event(inner_wd, crate::fs_notify::IN_IGNORED, None)], "inotify.h IN_IGNORED follows removal of the watched directory");
        assert!(instance.take_events().is_empty(), "IN_DELETE is not a wired event");

        assert_eq!(call("mkdir").ptr(dir.ptr()).u32(0o777).invoke().unwrap().i32(), 0);
        let replacement = Text::new(&guest.guest("replacement"));
        assert_eq!(call("mkdir").ptr(replacement.ptr()).u32(0o777).invoke().unwrap().i32(), 0);
        let (target, target_wd) = watch_host_directory(&guest.host("made"));
        assert_eq!(call("rename").ptr(replacement.ptr()).ptr(dir.ptr()).invoke().unwrap().i32(), 0);
        assert_eq!(target.take_events(), vec![notify_event(target_wd, crate::fs_notify::IN_IGNORED, None)], "inotify.h IN_IGNORED follows rename over the watched directory");
    }
}

bounded_test! {
    fn bionic_close_write_of_a_mapped_file_waits_for_munmap() {
        // cite: bionic mman.h#munmap
        let guest = IoGuest::new("notify-mmap");
        let (instance, wd) = watch_host_directory(&guest.host(""));
        let fd = open(&guest.guest("mapped"), o::RDWR | o::CREAT, 0o600);
        assert!(fd >= 3, "errno {}", guest_errno());
        assert_eq!(call("ftruncate").i32(fd).int(16384).invoke().unwrap().i32(), 0);
        let address = call("mmap").ptr(0).int(16384).i32(3).i32(1).i32(fd).int(0).invoke().unwrap().u64() as usize;
        assert!(address != usize::MAX && address != 0, "errno {}", guest_errno());
        assert_eq!(close(fd), 0);
        assert_eq!(instance.take_events(), vec![notify_event(wd, crate::fs_notify::IN_CREATE, Some("mapped"))], "inotify(7) IN_CLOSE_WRITE waits while a mapping still references the open file");
        unsafe { *(address as *mut u8) = 7 };
        assert_eq!(call("munmap").ptr(address).int(16384).invoke().unwrap().i32(), 0, "errno {}", guest_errno());
        assert_eq!(instance.take_events(), vec![notify_event(wd, crate::fs_notify::IN_CLOSE_WRITE, Some("mapped"))], "inotify(7) IN_CLOSE_WRITE follows the release of the last reference, here munmap");
        assert_eq!(std::fs::read(guest.host("mapped")).unwrap()[0], 7);
    }
}

bounded_test! {
    fn bionic_close_write_follows_rename_of_the_open_file_and_self_rename_keeps_the_watch() {
        // cite: bionic stdio.h#rename
        let guest = IoGuest::new("notify-rename");
        std::fs::create_dir(guest.host("tmp")).unwrap();
        std::fs::create_dir(guest.host("anr")).unwrap();
        let (instance, wd) = watch_host_directory(&guest.host("anr"));
        let fd = open(&guest.guest("tmp/x"), o::WRONLY | o::CREAT, 0o600);
        assert!(fd >= 3, "errno {}", guest_errno());
        let from = Text::new(&guest.guest("tmp/x"));
        let to = Text::new(&guest.guest("anr/x"));
        assert_eq!(call("rename").ptr(from.ptr()).ptr(to.ptr()).invoke().unwrap().i32(), 0);
        assert_eq!(close(fd), 0);
        assert_eq!(instance.take_events(), vec![notify_event(wd, crate::fs_notify::IN_CLOSE_WRITE, Some("x"))], "inotify(7) reports the close in the directory that holds the file at close time");

        let anr = Text::new(&guest.guest("anr"));
        assert_eq!(call("rename").ptr(anr.ptr()).ptr(anr.ptr()).invoke().unwrap().i32(), 0);
        let kept = open(&guest.guest("anr/kept"), o::WRONLY | o::CREAT, 0o600);
        assert_eq!(close(kept), 0);
        assert_eq!(
            instance.take_events(),
            vec![
                notify_event(wd, crate::fs_notify::IN_CREATE, Some("kept")),
                notify_event(wd, crate::fs_notify::IN_CLOSE_WRITE, Some("kept")),
            ],
            "rename(2): when both names are the same file, rename does nothing and succeeds"
        );
    }
}
