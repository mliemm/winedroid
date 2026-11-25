use super::test_deadline::bounded_test;
use super::thread_test_support::{each_variant, shim};

bounded_test! {
    fn bionic_thread_local_variable_has_a_private_copy_per_guest_thread() {
        // cite: bionic bionic_elf_tls.cpp#__tls_get_addr
        each_variant(|bridged| {
            assert_eq!(bridged.call("fx_tls_variable_per_thread").i32(6).invoke().unwrap().i32(), 6 + 100);
        });
    }
}

fn tls_index(module: u64, offset: u64) -> usize {
    Box::leak(Box::new([module, offset])).as_ptr() as usize
}

bounded_test! {
    fn bionic_tls_get_addr_gives_each_thread_a_private_block_copied_from_the_template() {
        // cite: bionic bionic_elf_tls.cpp#__tls_get_addr
        use super::elf_image::TlsSegment;
        let template = Box::leak(Box::new([1u8, 2, 3, 4, 9, 9, 9, 9])).as_ptr() as usize;
        let module = super::tls_modules::register(TlsSegment { size: 8, alignment: 8, init_vaddr: 0, init_size: 4 }, template);
        let index = tls_index(u64::from(module), 2);

        let mine = shim("__tls_get_addr").ptr(index).invoke().unwrap().u64() as usize;
        assert_eq!(unsafe { std::slice::from_raw_parts((mine - 2) as *const u8, 8).to_vec() }, vec![1, 2, 3, 4, 0, 0, 0, 0]);
        unsafe { *(mine as *mut u8) = 77 };
        let other = std::thread::spawn(move || {
            let address = shim("__tls_get_addr").ptr(index).invoke().unwrap().u64() as usize;
            (address, unsafe { *(address as *const u8) })
        })
        .join()
        .unwrap();

        assert_ne!(other.0, mine);
        assert_eq!(other.1, 3);
        assert_eq!(shim("__tls_get_addr").ptr(index).invoke().unwrap().u64() as usize, mine, "the block is stable for the thread");
        super::tls_modules::unregister(module);
    }
}

bounded_test! {
    fn bionic_tls_get_addr_for_an_unregistered_module_fails_explicitly() {
        // cite: bionic bionic_elf_tls.cpp#__tls_get_addr
        let error = shim("__tls_get_addr").ptr(tls_index(60_000, 0)).invoke().unwrap_err().to_string();

        assert!(error.contains("TLS module 60000 is not registered"), "{error}");
    }
}
