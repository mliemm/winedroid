#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct Regs {
    pub x: [u64; 8],
    pub x8: u64,
    pub lr: u64,
    pub sp: u64,
    pub fp: u64,
    pub q: [[u64; 2]; 8],
}

pub const REGS_SIZE: usize = 224;

const _: () = assert!(std::mem::size_of::<Regs>() == REGS_SIZE);
const _: () = assert!(std::mem::offset_of!(Regs, x8) == 64);
const _: () = assert!(std::mem::offset_of!(Regs, lr) == 72);
const _: () = assert!(std::mem::offset_of!(Regs, sp) == 80);
const _: () = assert!(std::mem::offset_of!(Regs, fp) == 88);
const _: () = assert!(std::mem::offset_of!(Regs, q) == 96);

impl Regs {
    pub fn zeroed() -> Self {
        Self {
            x: [0; 8],
            x8: 0,
            lr: 0,
            sp: 0,
            fp: 0,
            q: [[0; 2]; 8],
        }
    }

    pub fn arg_u64(&self, index: usize) -> u64 {
        self.x[index]
    }

    pub fn arg_i32(&self, index: usize) -> i32 {
        self.x[index] as i32
    }

    pub fn arg_u32(&self, index: usize) -> u32 {
        self.x[index] as u32
    }

    pub fn arg_ptr(&self, index: usize) -> usize {
        self.x[index] as usize
    }

    pub fn arg_f64(&self, index: usize) -> f64 {
        f64::from_bits(self.q[index][0])
    }

    pub fn arg_f32(&self, index: usize) -> f32 {
        f32::from_bits(self.q[index][0] as u32)
    }

    pub fn arg_quad(&self, index: usize) -> u128 {
        u128::from(self.q[index][0]) | (u128::from(self.q[index][1]) << 64)
    }

    pub fn ret_i32(&mut self, value: i32) {
        self.x[0] = i64::from(value) as u64;
    }

    pub fn ret_u32(&mut self, value: u32) {
        self.x[0] = u64::from(value);
    }

    pub fn ret_i64(&mut self, value: i64) {
        self.x[0] = value as u64;
    }

    pub fn ret_u64(&mut self, value: u64) {
        self.x[0] = value;
    }

    pub fn ret_ptr(&mut self, value: usize) {
        self.x[0] = value as u64;
    }

    pub fn ret_pair(&mut self, first: u64, second: u64) {
        self.x[0] = first;
        self.x[1] = second;
    }

    pub fn ret_f32(&mut self, value: f32) {
        self.q[0] = [u64::from(value.to_bits()), 0];
    }

    pub fn ret_f64(&mut self, value: f64) {
        self.q[0] = [value.to_bits(), 0];
    }

    pub fn ret_quad(&mut self, value: u128) {
        self.q[0] = [value as u64, (value >> 64) as u64];
    }

    pub fn ret_hfa_f32(&mut self, members: &[f32]) {
        for (register, member) in members.iter().enumerate() {
            self.q[register] = [u64::from(member.to_bits()), 0];
        }
    }

    pub fn ret_sret(&mut self, bytes: &[u8]) {
        assert_ne!(self.x8, 0, "indirect result location is null");
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.x8 as *mut u8, bytes.len()) };
        self.x[0] = self.x8;
    }
}
