use crate::error::Result;

use super::guest_call::{invoke_frame, GuestResult, SIGNAL_HANDLER_FRAME};
use super::trampoline::GuestCallFrame;

const GENERAL_REGISTERS: usize = 8;
const VECTOR_REGISTERS: usize = 8;
const COMPOSITE_REGISTER_LIMIT: usize = 16;

pub struct CallBuilder {
    frame: GuestCallFrame,
    next_general: usize,
    next_vector: usize,
    stack: Vec<u64>,
    copies: Vec<Box<[u128]>>,
}

impl CallBuilder {
    pub fn new(function: usize) -> Self {
        let frame = GuestCallFrame {
            function: function as u64,
            x: [0; 8],
            x8: 0,
            q: [[0; 2]; 8],
            stack_words: 0,
            stack: 0,
            ret_x: [0; 2],
            ret_q: [[0; 2]; 4],
            frame: 0,
            aborted: 0,
            stack_top: 0,
            serial: 0,
        };
        Self {
            frame,
            next_general: 0,
            next_vector: 0,
            stack: Vec::new(),
            copies: Vec::new(),
        }
    }

    fn pad_stack_to_sixteen(&mut self) {
        if self.stack.len() % 2 == 1 {
            self.stack.push(0);
        }
    }

    pub fn int(mut self, value: u64) -> Self {
        if self.next_general < GENERAL_REGISTERS {
            self.frame.x[self.next_general] = value;
            self.next_general += 1;
        } else {
            self.stack.push(value);
        }
        self
    }

    pub fn ptr(self, value: usize) -> Self {
        self.int(value as u64)
    }

    pub fn i8(self, value: i8) -> Self {
        self.int(i64::from(value) as u64)
    }

    pub fn i16(self, value: i16) -> Self {
        self.int(i64::from(value) as u64)
    }

    pub fn i32(self, value: i32) -> Self {
        self.int(i64::from(value) as u64)
    }

    pub fn u8(self, value: u8) -> Self {
        self.int(u64::from(value))
    }

    pub fn u16(self, value: u16) -> Self {
        self.int(u64::from(value))
    }

    pub fn u32(self, value: u32) -> Self {
        self.int(u64::from(value))
    }

    pub fn f64(mut self, value: f64) -> Self {
        if self.next_vector < VECTOR_REGISTERS {
            self.frame.q[self.next_vector] = [value.to_bits(), 0];
            self.next_vector += 1;
        } else {
            self.stack.push(value.to_bits());
        }
        self
    }

    pub fn f32(mut self, value: f32) -> Self {
        if self.next_vector < VECTOR_REGISTERS {
            self.frame.q[self.next_vector] = [u64::from(value.to_bits()), 0];
            self.next_vector += 1;
        } else {
            self.stack.push(u64::from(value.to_bits()));
        }
        self
    }

    pub fn variadic_f32(self, value: f32) -> Self {
        self.f64(f64::from(value))
    }

    pub fn quad(mut self, value: u128) -> Self {
        if self.next_vector < VECTOR_REGISTERS {
            self.frame.q[self.next_vector] = [value as u64, (value >> 64) as u64];
            self.next_vector += 1;
        } else {
            self.pad_stack_to_sixteen();
            self.stack.push(value as u64);
            self.stack.push((value >> 64) as u64);
        }
        self
    }

    pub fn int128(mut self, value: u128) -> Self {
        self.next_general += self.next_general % 2;
        if self.next_general + 2 <= GENERAL_REGISTERS {
            self.frame.x[self.next_general] = value as u64;
            self.frame.x[self.next_general + 1] = (value >> 64) as u64;
            self.next_general += 2;
        } else {
            self.next_general = GENERAL_REGISTERS;
            self.pad_stack_to_sixteen();
            self.stack.push(value as u64);
            self.stack.push((value >> 64) as u64);
        }
        self
    }

    pub fn sret(mut self, destination: *mut u8) -> Self {
        self.frame.x8 = destination as u64;
        self
    }

    pub fn composite(mut self, bytes: &[u8], align: usize, hfa: Option<(usize, usize)>) -> Self {
        let words = pack_words(bytes);
        if let Some((element, count)) = hfa {
            if self.next_vector + count <= VECTOR_REGISTERS {
                for member in 0..count {
                    let mut register = [0u8; 16];
                    register[..element]
                        .copy_from_slice(&bytes[member * element..(member + 1) * element]);
                    self.frame.q[self.next_vector + member] = [
                        u64::from_le_bytes(register[..8].try_into().expect("8 bytes")),
                        u64::from_le_bytes(register[8..].try_into().expect("8 bytes")),
                    ];
                }
                self.next_vector += count;
            } else {
                self.next_vector = VECTOR_REGISTERS;
                self.stack.extend(words);
            }
            return self;
        }
        if bytes.len() > COMPOSITE_REGISTER_LIMIT {
            let mut copy = vec![0u128; bytes.len().div_ceil(16)].into_boxed_slice();
            unsafe {
                std::ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    copy.as_mut_ptr() as *mut u8,
                    bytes.len(),
                )
            };
            let pointer = copy.as_ptr() as usize;
            self.copies.push(copy);
            return self.ptr(pointer);
        }
        if align == 16 {
            self.next_general += self.next_general % 2;
        }
        if self.next_general + words.len() <= GENERAL_REGISTERS {
            for word in words {
                self.frame.x[self.next_general] = word;
                self.next_general += 1;
            }
        } else {
            self.next_general = GENERAL_REGISTERS;
            if align == 16 {
                self.pad_stack_to_sixteen();
            }
            self.stack.extend(words);
        }
        self
    }

    pub fn on_stack(mut self, top: usize) -> Self {
        self.frame.stack_top = top as u64;
        self
    }

    pub fn signal_handler(mut self) -> Self {
        self.frame.serial = SIGNAL_HANDLER_FRAME;
        self
    }

    pub fn invoke(mut self) -> Result<GuestResult> {
        self.frame.stack_words = self.stack.len() as u64;
        self.frame.stack = self.stack.as_ptr() as u64;
        let outcome = invoke_frame(&mut self.frame);
        drop(self.copies);
        outcome
    }
}

fn pack_words(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks(8)
        .map(|chunk| {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            u64::from_le_bytes(word)
        })
        .collect()
}
