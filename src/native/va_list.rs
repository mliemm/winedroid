use std::marker::PhantomData;

use crate::error::{Result, WinedroidError};

use super::guest_regs::Regs;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GuestVaListRaw {
    pub stack: u64,
    pub gr_top: u64,
    pub vr_top: u64,
    pub gr_offs: i32,
    pub vr_offs: i32,
}

const _: () = assert!(std::mem::size_of::<GuestVaListRaw>() == 32);

#[derive(Debug, Clone, Copy)]
pub struct GuestVaList<'a> {
    raw: GuestVaListRaw,
    owner: PhantomData<&'a Regs>,
}

fn read_word(address: u64) -> u64 {
    unsafe { std::ptr::read_unaligned(address as *const u64) }
}

impl GuestVaList<'static> {
    pub fn from_guest(pointer: u64) -> Result<Self> {
        if pointer == 0 {
            return Err(malformed(pointer, "null va_list"));
        }
        let raw = unsafe { std::ptr::read_unaligned(pointer as *const GuestVaListRaw) };
        if raw.gr_offs % 8 != 0 || raw.gr_offs < -64 {
            return Err(malformed(pointer, "gr_offs"));
        }
        if raw.vr_offs % 16 != 0 || raw.vr_offs < -128 {
            return Err(malformed(pointer, "vr_offs"));
        }
        if (raw.gr_offs < 0 && raw.gr_top == 0) || (raw.vr_offs < 0 && raw.vr_top == 0) {
            return Err(malformed(pointer, "register save area"));
        }
        Ok(Self { raw, owner: PhantomData })
    }
}

fn malformed(pointer: u64, what: &str) -> WinedroidError {
    WinedroidError::new(format!("malformed va_list at {pointer:#x}: {what}; next missing primitive: malformed va_list"))
}

impl<'a> GuestVaList<'a> {
    pub fn from_regs(regs: &'a Regs, named_gp: u32, named_fp: u32) -> Self {
        let gr_top = std::ptr::addr_of!(regs.x8) as u64;
        let vr_top = (std::ptr::addr_of!(regs.q) as u64).wrapping_add(128);
        Self {
            raw: GuestVaListRaw {
                stack: regs.sp,
                gr_top,
                vr_top,
                gr_offs: -((8 - named_gp.min(8) as i32) * 8),
                vr_offs: -((8 - named_fp.min(8) as i32) * 16),
            },
            owner: PhantomData,
        }
    }

    pub fn raw(&self) -> GuestVaListRaw {
        self.raw
    }

    fn register_slot(&mut self, slots: i32, vector: bool) -> Option<u64> {
        let (offs, top, step) = if vector {
            (&mut self.raw.vr_offs, self.raw.vr_top, 16)
        } else {
            (&mut self.raw.gr_offs, self.raw.gr_top, 8)
        };
        let off = *offs;
        if off >= 0 {
            return None;
        }
        let next = off.wrapping_add(step * slots);
        *offs = next;
        if next > 0 {
            return None;
        }
        Some(top.wrapping_add(off as i64 as u64))
    }

    fn stack_slot(&mut self, size: u64, align: u64) -> u64 {
        let at = self.raw.stack.wrapping_add(align - 1) & !(align - 1);
        self.raw.stack = at.wrapping_add(size.div_ceil(8) * 8);
        at
    }

    pub fn next_gp(&mut self) -> u64 {
        let address = self.register_slot(1, false).unwrap_or_else(|| self.stack_slot(8, 8));
        read_word(address)
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_gp() as i32
    }

    pub fn next_u32(&mut self) -> u32 {
        self.next_gp() as u32
    }

    pub fn next_i64(&mut self) -> i64 {
        self.next_gp() as i64
    }

    pub fn next_ptr(&mut self) -> usize {
        self.next_gp() as usize
    }

    pub fn next_f64(&mut self) -> f64 {
        let address = self.register_slot(1, true).unwrap_or_else(|| self.stack_slot(8, 8));
        f64::from_bits(read_word(address))
    }

    pub fn next_quad(&mut self) -> u128 {
        let address = self.register_slot(1, true).unwrap_or_else(|| self.stack_slot(16, 16));
        read_quad(address)
    }

    pub fn next_u128(&mut self) -> u128 {
        if self.raw.gr_offs < 0 {
            self.raw.gr_offs = self.raw.gr_offs.wrapping_add(15) & !15;
        }
        let address = self.register_slot(2, false).unwrap_or_else(|| self.stack_slot(16, 16));
        read_quad(address)
    }

    pub fn next_composite(&mut self, size: usize, align: usize, hfa: Option<(usize, usize)>) -> Vec<u8> {
        if size > 16 && hfa.is_none() {
            let pointer = self.next_gp();
            return unsafe { std::slice::from_raw_parts(pointer as *const u8, size) }.to_vec();
        }
        let mut bytes = Vec::with_capacity(size);
        if let Some((element, count)) = hfa {
            match self.register_slot(count as i32, true) {
                Some(start) => {
                    for index in 0..count {
                        let source = start.wrapping_add(16 * index as u64);
                        bytes.extend_from_slice(unsafe { std::slice::from_raw_parts(source as *const u8, element) });
                    }
                }
                None => {
                    let at = self.stack_slot(size as u64, 8);
                    bytes.extend_from_slice(unsafe { std::slice::from_raw_parts(at as *const u8, size) });
                }
            }
            return bytes;
        }
        let slots = size.div_ceil(8) as i32;
        if align == 16 && self.raw.gr_offs < 0 {
            self.raw.gr_offs = self.raw.gr_offs.wrapping_add(15) & !15;
        }
        let address = self.register_slot(slots, false).unwrap_or_else(|| self.stack_slot(size as u64, align.max(8) as u64));
        bytes.extend_from_slice(unsafe { std::slice::from_raw_parts(address as *const u8, size) });
        bytes
    }
}

fn read_quad(address: u64) -> u128 {
    unsafe { std::ptr::read_unaligned(address as *const u128) }
}
