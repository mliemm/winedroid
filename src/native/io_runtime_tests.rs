use std::ffi::CString;

use super::bridge_test_support::Bridged;
use super::dl_api::DlHandle;
use super::io_test_support::{call, IoGuest};
use super::libc_core_tests::c_text;
use super::loader_test_support::{child_outcome, fixture_dir};
use super::runtime::NativeRuntime;
use super::test_deadline::bounded_test;

fn io_fixture() -> (IoGuest, Bridged) {
    let guest = IoGuest::new("io-runtime");
    let bridged = Bridged::load("16k", "libfx_io.so");
    (guest, bridged)
}

fn net_fixture() -> (IoGuest, Bridged) {
    let guest = IoGuest::new("net-runtime");
    let bridged = Bridged::load("16k", "libfx_net.so");
    (guest, bridged)
}

fn text_argument(text: &str) -> CString {
    CString::new(text).unwrap()
}

bounded_test! {
    fn fixture_printf_family_matches_the_bionic_conversion_table() {
        // cite: bionic vfprintf.cpp#vfprintf
        let (_guest, bridged) = io_fixture();
        let mut buffer = vec![0u8; 512];
        let written = bridged.call("fx_printf_formats").ptr(buffer.as_mut_ptr() as usize).i32(512).invoke().unwrap().i32();
        let expected = "42|-9|0x1234|str|z|ff|10|0x1p+0|1.23e+03|0.0001";
        assert_eq!(c_text(&buffer), expected);
        assert_eq!(written as usize, expected.len());
    }
}

bounded_test! {
    fn fixture_positional_printf_reads_arguments_by_index() {
        // cite: bionic vfprintf.cpp#vfprintf
        let (_guest, bridged) = io_fixture();
        let mut buffer = vec![0u8; 128];
        let written = bridged.call("fx_printf_positional").ptr(buffer.as_mut_ptr() as usize).i32(128).invoke().unwrap().i32();
        let expected = "seven=7|2.500";
        assert_eq!(c_text(&buffer), expected);
        assert_eq!(written as usize, expected.len());
    }
}

bounded_test! {
    fn fixture_stdio_file_roundtrip_uses_a_real_file_object() {
        // cite: bionic stdio.cpp#fopen, __sFILE
        let (guest, bridged) = io_fixture();
        let path = text_argument(&guest.guest("stdio.txt"));
        assert_eq!(bridged.call("fx_stdio_roundtrip").ptr(path.as_ptr() as usize).invoke().unwrap().i32(), 1);
        assert_eq!(std::fs::read(guest.host("stdio.txt")).unwrap(), b"7|seven|2.50|x\n");
    }
}

bounded_test! {
    fn fixture_getline_reads_buffered_lines_and_reports_end_of_file() {
        // cite: bionic getdelim.c#getdelim
        let (guest, bridged) = io_fixture();
        let path = text_argument(&guest.guest("lines.txt"));
        assert_eq!(bridged.call("fx_stdio_getline").ptr(path.as_ptr() as usize).invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn fixture_ungetc_pushes_one_byte_back_into_the_stream() {
        // cite: bionic stdio.cpp#ungetc
        let (guest, bridged) = io_fixture();
        let path = text_argument(&guest.guest("ungetc.txt"));
        assert_eq!(bridged.call("fx_stdio_ungetc").ptr(path.as_ptr() as usize).invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn fixture_sscanf_covers_the_integer_and_float_families() {
        // cite: bionic vfscanf.cpp#vfscanf
        let (_guest, bridged) = io_fixture();
        let input = text_argument("42 -17 ff 17 4000000000 2.5 3.25 word Q");
        assert_eq!(bridged.call("fx_scanf_sscanf").ptr(input.as_ptr() as usize).invoke().unwrap().i32(), 0x1ff);
    }
}

bounded_test! {
    fn fixture_sscanf_respects_the_width_limit() {
        // cite: bionic vfscanf.cpp#vfscanf
        let (_guest, bridged) = io_fixture();
        assert_eq!(bridged.call("fx_scanf_width").invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn fixture_sscanf_count_pointer_reports_consumed_characters_without_counting_as_an_assignment() {
        // cite: bionic vfscanf.cpp#vfscanf
        let (_guest, bridged) = io_fixture();
        assert_eq!(bridged.call("fx_scanf_count_pointer").invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn fixture_dirent_lists_entries_created_in_the_guest_prefix() {
        // cite: bionic dirent.cpp#readdir
        let (guest, bridged) = io_fixture();
        let directory = guest.host("listing");
        std::fs::create_dir(&directory).unwrap();
        for name in ["alpha", "beta", "gamma"] {
            std::fs::write(directory.join(name), b"x").unwrap();
        }
        let path = text_argument(&guest.guest("listing"));
        assert_eq!(bridged.call("fx_dirent_count").ptr(path.as_ptr() as usize).invoke().unwrap().i32(), 3);
    }
}

bounded_test! {
    fn fixture_stat_reports_the_size_of_a_file_in_the_guest_prefix() {
        // cite: bionic stat.h#__STAT64_BODY
        let (guest, bridged) = io_fixture();
        std::fs::write(guest.host("size.bin"), b"hello, world").unwrap();
        let path = text_argument(&guest.guest("size.bin"));
        assert_eq!(bridged.call("fx_stat_size").ptr(path.as_ptr() as usize).invoke().unwrap().i32(), 12);
    }
}

bounded_test! {
    fn fixture_mmap_of_anonymous_memory_is_writable_and_can_become_read_only() {
        // cite: bionic mmap.cpp#mmap
        let (_guest, bridged) = io_fixture();
        assert_eq!(bridged.call("fx_mmap_anon").invoke().unwrap().i32(), 120);
    }
}

bounded_test! {
    fn fixture_mmap_of_a_file_reads_the_first_bytes_through_the_guest_descriptor() {
        // cite: bionic mmap.cpp#mmap
        let (guest, bridged) = io_fixture();
        std::fs::write(guest.host("map.bin"), b"hello world").unwrap();
        let path = text_argument(&guest.guest("map.bin"));
        assert_eq!(bridged.call("fx_mmap_file").ptr(path.as_ptr() as usize).invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn fixture_proc_self_maps_lists_the_loaded_guest_libraries() {
        // cite: bionic pthread_attr.cpp#__get_main_stack_startstack
        let _guest = IoGuest::new("maps");
        let runtime = NativeRuntime::initialize(vec![fixture_dir("16k")]).unwrap();
        let handle = runtime.loader().dlopen(Some("libfx_io.so"), 2, None).unwrap();
        let address = runtime.loader().dlsym(DlHandle::Module(handle), "fx_maps_contains", None).unwrap();
        let library = text_argument("libfx_io.so");
        let rwx = text_argument(" r-xp ");
        let count = runtime.call_guest(address, &[library.as_ptr() as u64]).unwrap().i32();
        assert!(count >= 1, "the loaded fixture must appear in /proc/self/maps");
        assert!(runtime.call_guest(address, &[rwx.as_ptr() as u64]).unwrap().i32() >= 1, "the maps lines carry Linux permission flags");
        let data = text_argument("/data/app/");
        assert!(runtime.call_guest(address, &[data.as_ptr() as u64]).unwrap().i32() >= 1, "the library name is the guest path");
    }
}

bounded_test! {
    fn fixture_swprintf_writes_wide_output_with_wide_string_arguments() {
        // cite: bionic vfprintf.cpp#vfprintf
        let (_guest, bridged) = io_fixture();
        let mut units = vec![0u32; 16];
        let written = bridged.call("fx_swprintf_wide").ptr(units.as_mut_ptr() as usize).i32(16).invoke().unwrap().i32();
        assert_eq!(written, 6);
        let text: String = units.iter().take_while(|unit| **unit != 0).map(|unit| char::from_u32(*unit).unwrap_or('\u{fffd}')).collect();
        assert_eq!(text, "5-wide");
    }
}

bounded_test! {
    fn fixture_tmpfile_roundtrip_uses_an_anonymous_file() {
        // cite: bionic tmpfile.cpp#tmpfile
        let (_guest, bridged) = io_fixture();
        assert_eq!(bridged.call("fx_tmpfile_roundtrip").invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn fixture_loopback_tcp_client_and_server_exchange_messages() {
        // cite: bionic socket.h#socket, connect, accept
        let (_guest, bridged) = net_fixture();
        assert_eq!(bridged.call("fx_tcp_roundtrip").invoke().unwrap().i32(), 15);
    }
}

bounded_test! {
    fn fixture_epoll_reports_level_triggered_readiness_repeatedly() {
        // cite: bionic sys_epoll.cpp#epoll_wait
        let (_guest, bridged) = net_fixture();
        assert_eq!(bridged.call("fx_epoll_level").invoke().unwrap().i32(), 7);
    }
}

bounded_test! {
    fn fixture_epoll_edge_triggered_reports_only_new_transitions() {
        // cite: bionic epoll.h#epoll_ctl
        let (_guest, bridged) = net_fixture();
        assert_eq!(bridged.call("fx_epoll_edge").invoke().unwrap().i32(), 7);
    }
}

bounded_test! {
    fn fixture_epoll_oneshot_requires_rearming_with_epoll_ctl_mod() {
        // cite: bionic epoll.h#epoll_ctl
        let (_guest, bridged) = net_fixture();
        assert_eq!(bridged.call("fx_epoll_oneshot").invoke().unwrap().i32(), 7);
    }
}

bounded_test! {
    fn fixture_eventfd_roundtrips_a_counter_and_pipe_poll_and_select_see_readiness() {
        // cite: bionic eventfd.cpp#eventfd_read
        // cite: bionic poll.cpp#poll, select
        let (_guest, bridged) = net_fixture();
        assert_eq!(bridged.call("fx_eventfd_value").invoke().unwrap().i32(), 7);
        assert_eq!(bridged.call("fx_pipe_poll").invoke().unwrap().i32(), 3);
        assert_eq!(bridged.call("fx_select_readable").invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn fixture_getaddrinfo_resolves_localhost_and_getnameinfo_formats_the_result() {
        // cite: bionic getaddrinfo.c#getaddrinfo
        // cite: bionic getnameinfo.c#getnameinfo
        let (_guest, bridged) = net_fixture();
        assert_eq!(bridged.call("fx_getaddrinfo_localhost").invoke().unwrap().i32(), 15);
        assert_eq!(bridged.call("fx_getnameinfo_numeric").invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn fixture_timerfd_delivers_repeated_expirations() {
        // cite: bionic timerfd.h#TFD_TIMER_ABSTIME
        let (_guest, bridged) = net_fixture();
        assert_eq!(bridged.call("fx_timerfd_tick").invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn fixture_socketpair_shutdown_reports_end_of_file_and_setsockopt_roundtrips() {
        // cite: bionic socket.h#shutdown, setsockopt
        let (_guest, bridged) = net_fixture();
        assert_eq!(bridged.call("fx_socketpair_shutdown").invoke().unwrap().i32(), 3);
        assert_eq!(bridged.call("fx_setsockopt_roundtrip").invoke().unwrap().i32(), 1);
    }
}

bounded_test! {
    fn unsupported_inotify_and_signalfd_fail_with_an_explicit_missing_primitive() {
        // cite: bionic inotify.h#inotify_init1
        // cite: bionic signalfd.h#signalfd
        let (_, stderr) = child_outcome(|| {
            assert_eq!(call("inotify_init1").i32(0).invoke().unwrap().i32(), -1);
            assert_eq!(call("signalfd").i32(-1).ptr(0).i32(0).invoke().unwrap().i32(), -1);
        });
        assert!(stderr.contains("FIXME: missing-primitive inotify_init1"), "{stderr}");
        assert!(stderr.contains("FIXME: missing-primitive signalfd"), "{stderr}");
    }
}
