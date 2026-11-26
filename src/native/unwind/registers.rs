use super::{refuse, CONTEXT_MAGIC};

pub const GENERAL_REGISTER_COUNT: usize = 31;
pub const STACK_POINTER_REGISTER: u32 = 31;
pub const VECTOR_FIRST_REGISTER: u32 = 72;
pub const VECTOR_REGISTER_COUNT: usize = 8;
pub const LINK_REGISTER: u32 = 30;
pub const SHADOW_CALL_STACK_REGISTER: usize = 18;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnwindRegs {
    pub q: [u128; VECTOR_REGISTER_COUNT],
    pub x: [u64; GENERAL_REGISTER_COUNT],
    pub sp: u64,
    pub pc: u64,
}

const _: () = assert!(std::mem::size_of::<UnwindRegs>() == 400);
const _: () = assert!(std::mem::offset_of!(UnwindRegs, x) == 128);
const _: () = assert!(std::mem::offset_of!(UnwindRegs, sp) == 376);
const _: () = assert!(std::mem::offset_of!(UnwindRegs, pc) == 384);

impl UnwindRegs {
    pub const fn zeroed() -> Self {
        Self {
            q: [0; VECTOR_REGISTER_COUNT],
            x: [0; GENERAL_REGISTER_COUNT],
            sp: 0,
            pc: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct UnwindContext {
    pub magic: u64,
    pub regs: UnwindRegs,
    pub cfa: u64,
    pub region_start: u64,
    pub lsda: u64,
    pub fde: u64,
    pub personality: u64,
    pub handler_frame: bool,
}

impl UnwindContext {
    pub fn new(regs: UnwindRegs) -> Self {
        Self {
            magic: CONTEXT_MAGIC,
            regs,
            cfa: 0,
            region_start: 0,
            lsda: 0,
            fde: 0,
            personality: 0,
            handler_frame: false,
        }
    }

    pub fn from_guest(pointer: usize) -> &'static mut Self {
        if pointer == 0 {
            refuse("next missing primitive: cross-unwinder _Unwind_Context");
        }
        let context = unsafe { &mut *(pointer as *mut Self) };
        if context.magic != CONTEXT_MAGIC {
            refuse(&format!(
                "unwind accessor received a context that was not created by this unwinder (magic {:#x}); next missing primitive: cross-unwinder _Unwind_Context",
                context.magic
            ));
        }
        context
    }

    pub fn register(&self, register: u32) -> Option<u64> {
        match register {
            0..=30 => Some(self.regs.x[register as usize]),
            STACK_POINTER_REGISTER => Some(self.regs.sp),
            VECTOR_FIRST_REGISTER..=79 => {
                Some(self.regs.q[(register - VECTOR_FIRST_REGISTER) as usize] as u64)
            }
            _ => None,
        }
    }

    pub fn set_register(&mut self, register: u32, value: u64) {
        match register {
            0..=30 => self.regs.x[register as usize] = value,
            STACK_POINTER_REGISTER => self.regs.sp = value,
            VECTOR_FIRST_REGISTER..=79 => {
                let slot = &mut self.regs.q[(register - VECTOR_FIRST_REGISTER) as usize];
                *slot = (*slot & !u128::from(u64::MAX)) | u128::from(value);
            }
            _ => refuse(&format!("unwind accessor for unknown register {register}; next missing primitive: DWARF register {register}")),
        }
    }
}

pub fn register_value(regs: &UnwindRegs, register: u32) -> Option<u64> {
    match register {
        0..=30 => Some(regs.x[register as usize]),
        STACK_POINTER_REGISTER => Some(regs.sp),
        VECTOR_FIRST_REGISTER..=79 => {
            Some(regs.q[(register - VECTOR_FIRST_REGISTER) as usize] as u64)
        }
        _ => None,
    }
}
