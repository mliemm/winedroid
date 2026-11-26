use super::bionic::syscall::translate;
use super::test_deadline::bounded_test;
use super::thread_test_support::shim;

bounded_test! {
    fn prctl_set_name_feeds_the_thread_name_table_that_pr_get_name_and_pthread_getname_np_read() {
        // cite: bionic prctl.h#PR_SET_NAME, PR_GET_NAME
        let me = super::guest_thread::attach_current().unwrap() as usize;
        let long_name = Box::leak(Box::new(*b"worker-from-prctl-extra\0")).as_ptr() as usize;
        let (by_prctl, by_pthread) = (Box::leak(Box::new([0xAAu8; 16])).as_mut_ptr() as usize, Box::leak(Box::new([0xAAu8; 16])).as_mut_ptr() as usize);

        assert_eq!(shim("prctl").i32(15).ptr(long_name).int(0).int(0).int(0).invoke().unwrap().i32(), 0);
        assert_eq!(shim("prctl").i32(16).ptr(by_prctl).int(0).int(0).int(0).invoke().unwrap().i32(), 0);
        assert_eq!(shim("pthread_getname_np").ptr(me).ptr(by_pthread).int(16).invoke().unwrap().i32(), 0);

        let expected = b"worker-from-prc\0";
        assert_eq!(unsafe { std::slice::from_raw_parts(by_prctl as *const u8, 16) }, expected);
        assert_eq!(unsafe { std::slice::from_raw_parts(by_pthread as *const u8, 16) }, expected);
    }
}

bounded_test! {
    fn prctl_refuses_an_untranslated_option_with_einval_instead_of_pretending_success() {
        // cite: bionic SYSCALLS.TXT#prctl
        assert_eq!(shim("prctl").i32(9999).int(0).int(0).int(0).int(0).invoke().unwrap().i32(), -1);
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(22));
        assert_eq!(translate(167, [9999, 0, 0, 0, 0, 0]), -22);
    }
}
