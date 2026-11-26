use super::super::bridge::CallContext;
use super::super::call_assign::CallBuilder;
use super::super::unwind::entry::{
    spk_unwind_backtrace, spk_unwind_forced, spk_unwind_raise, spk_unwind_resume,
};
use super::super::unwind::fde;
use super::super::unwind::registers::UnwindContext;
use super::super::unwind::URC_FOREIGN_EXCEPTION_CAUGHT;
use super::export::{in_libc, in_libc_raw};
use super::guest_mem::read_value;

fn accessor(context: &mut CallContext) -> &'static mut UnwindContext {
    UnwindContext::from_guest(context.arg_ptr(0))
}

fn get_gr(context: &mut CallContext) {
    // cite: bionic libc.map.txt#_Unwind_GetGR
    let value = accessor(context).register(context.arg_u32(1)).unwrap_or(0);
    context.ret_u64(value);
}

fn set_gr(context: &mut CallContext) {
    // cite: bionic libc.map.txt#_Unwind_SetGR
    let (register, value) = (context.arg_u32(1), context.arg_u64(2));
    accessor(context).set_register(register, value);
}

fn get_ip(context: &mut CallContext) {
    // cite: bionic libc.map.txt#_Unwind_GetIP
    let pc = accessor(context).regs.pc;
    context.ret_u64(pc);
}

fn set_ip(context: &mut CallContext) {
    // cite: bionic libc.map.txt#_Unwind_SetIP
    let value = context.arg_u64(1);
    accessor(context).regs.pc = value;
}

pub fn register() {
    in_libc_raw(
        "_Unwind_RaiseException",
        spk_unwind_raise as *const () as usize,
    );
    in_libc_raw("_Unwind_Resume", spk_unwind_resume as *const () as usize);
    in_libc_raw(
        "_Unwind_ForcedUnwind",
        spk_unwind_forced as *const () as usize,
    );
    in_libc_raw(
        "_Unwind_Backtrace",
        spk_unwind_backtrace as *const () as usize,
    );
    in_libc("_Unwind_GetGR", get_gr);
    in_libc("_Unwind_SetGR", set_gr);
    in_libc("_Unwind_GetIP", get_ip);
    in_libc("_Unwind_SetIP", set_ip);
    in_libc("_Unwind_GetIPInfo", |context| {
        // cite: bionic libc.map.txt#_Unwind_GetIPInfo
        let (before, pc) = (context.arg_ptr(1), accessor(context).regs.pc);
        if before != 0 {
            super::guest_mem::write_value::<i32>(before, 0);
        }
        context.ret_u64(pc);
    });
    in_libc("_Unwind_GetCFA", |context| {
        // cite: bionic libc.map.txt#_Unwind_GetCFA
        let cfa = accessor(context).cfa;
        context.ret_u64(cfa);
    });
    in_libc("_Unwind_GetRegionStart", |context| {
        // cite: bionic libc.map.txt#_Unwind_GetRegionStart
        let start = accessor(context).region_start;
        context.ret_u64(start);
    });
    in_libc("_Unwind_GetLanguageSpecificData", |context| {
        // cite: bionic libc.map.txt#_Unwind_GetLanguageSpecificData
        let lsda = accessor(context).lsda;
        context.ret_u64(lsda);
    });
    in_libc("_Unwind_GetTextRelBase", |context| {
        // cite: bionic libc.map.txt#_Unwind_GetTextRelBase
        context.ret_u64(0);
    });
    in_libc("_Unwind_GetDataRelBase", |context| {
        // cite: bionic libc.map.txt#_Unwind_GetDataRelBase
        context.ret_u64(0);
    });
    in_libc("_Unwind_FindEnclosingFunction", |context| {
        // cite: bionic libc.map.txt#_Unwind_FindEnclosingFunction
        context.ret_u64(fde::find_enclosing(context.arg_u64(0)).unwrap_or(0));
    });
    in_libc("_Unwind_Find_FDE", |context| {
        // cite: bionic libc.map.txt#_Unwind_Find_FDE
        let pc = context.arg_u64(0);
        let bases = context.arg_ptr(1);
        let found = fde::lookup(pc).ok().flatten();
        let (address, function) = match &found {
            Some(found) => (found.fde.address as u64, found.fde.initial_location),
            None => (0, 0),
        };
        if bases != 0 {
            let block = [
                function.to_le_bytes(),
                0u64.to_le_bytes(),
                0u64.to_le_bytes(),
            ]
            .concat();
            super::guest_mem::write_bytes(bases, &block);
        }
        context.ret_u64(address);
    });
    in_libc("_Unwind_DeleteException", |context| {
        // cite: bionic libc.map.txt#_Unwind_DeleteException
        let exception = context.arg_ptr(0);
        if exception == 0 {
            return;
        }
        let cleanup = read_value::<u64>(exception + 8);
        if cleanup == 0 {
            return;
        }
        if let Err(error) = CallBuilder::new(cleanup as usize)
            .int(URC_FOREIGN_EXCEPTION_CAUGHT as u64)
            .ptr(exception)
            .invoke()
        {
            context.fail(error.to_string());
        }
    });
    in_libc("__register_frame", |context| {
        // cite: bionic libc.map.txt#__register_frame
        let begin = context.arg_ptr(0);
        if begin != 0 {
            fde::register_frame(begin);
        }
    });
    in_libc("__deregister_frame", |context| {
        // cite: bionic libc.map.txt#__deregister_frame
        let begin = context.arg_ptr(0);
        if begin != 0 {
            fde::deregister_frame(begin);
        }
    });
}
