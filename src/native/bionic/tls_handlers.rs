use super::super::tls_modules::block_address_checked;
use super::export::in_libc;
use super::guest_mem::read_value;

pub fn register() {
    in_libc("__tls_get_addr", |context| {
        // cite: bionic bionic_elf_tls.cpp#__tls_get_addr
        let index = context.arg_ptr(0);
        let (module, offset) = (read_value::<u64>(index), read_value::<u64>(index + 8));
        match u32::try_from(module).map_err(|_| format!("TLS module {module} is not registered; next missing primitive: TLS module {module}")).and_then(block_address_checked) {
            Ok(base) => context.ret_ptr(base + offset as usize),
            Err(message) => context.fail(message),
        }
    });
}
