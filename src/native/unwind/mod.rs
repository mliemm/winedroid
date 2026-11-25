pub mod cfi;
pub mod dwarf_expr;
pub mod entry;
pub mod fde;
pub mod registers;
pub mod walk;

pub const URC_NO_REASON: i32 = 0;
pub const URC_FOREIGN_EXCEPTION_CAUGHT: i32 = 1;
pub const URC_FATAL_PHASE2_ERROR: i32 = 2;
pub const URC_FATAL_PHASE1_ERROR: i32 = 3;
pub const URC_NORMAL_STOP: i32 = 4;
pub const URC_END_OF_STACK: i32 = 5;
pub const URC_HANDLER_FOUND: i32 = 6;
pub const URC_INSTALL_CONTEXT: i32 = 7;
pub const URC_CONTINUE_UNWIND: i32 = 8;

pub const UA_SEARCH_PHASE: i32 = 1;
pub const UA_CLEANUP_PHASE: i32 = 2;
pub const UA_HANDLER_FRAME: i32 = 4;
pub const UA_FORCE_UNWIND: i32 = 8;
pub const UA_END_OF_STACK: i32 = 16;

pub const UNWIND_VERSION: i32 = 1;
pub const CONTEXT_MAGIC: u64 = 0x5744_554e_5749_4e44;
pub const FRAME_BUDGET: usize = 4096;

pub fn refuse(message: &str) -> ! {
    super::bionic::fatal::fatal(None, message)
}
