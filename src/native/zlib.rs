use super::bridge::{register_handler_in, CallContext};

#[link(name = "z")]
extern "C" {
    fn adler32(adler: libc::c_ulong, buffer: *const u8, length: libc::c_uint) -> libc::c_ulong;
}

fn guest_adler32(context: &mut CallContext) {
    // cite: libcore Adler32.c#Java_java_util_zip_Adler32_updateBytes
    let checksum = unsafe {
        adler32(
            context.arg_u64(0),
            context.arg_ptr(1) as *const u8,
            context.arg_u32(2),
        )
    };
    context.ret_u64(checksum);
}

pub(super) fn register() {
    register_handler_in("libz.so", "adler32", guest_adler32);
}

#[cfg(test)]
mod tests {
    use crate::native::{
        bridge::BridgeProvider, call_assign::CallBuilder, handler_registry::register_all_groups,
        link_resolution::SystemLibraryProvider,
    };

    #[test]
    fn adler32_export_initializes_and_updates_the_checksum() {
        // cite: libcore Adler32.java#reset, update
        register_all_groups();
        let address = BridgeProvider.export("libz.so", "adler32").unwrap();
        let initial = CallBuilder::new(address)
            .int(0)
            .ptr(0)
            .u32(0)
            .invoke()
            .unwrap()
            .u64();
        assert_eq!(initial, 1);

        let bytes = b"123456789";
        let checksum = CallBuilder::new(address)
            .int(initial)
            .ptr(bytes.as_ptr() as usize)
            .u32(bytes.len() as u32)
            .invoke()
            .unwrap()
            .u64();
        assert_eq!(checksum, 0x091e01de);
    }
}
