use std::sync::Mutex;

use super::export::in_libc;
use super::guest_mem::{read_value, write_value};

const TYPE_0: i32 = 0;
const MAX_TYPES: i32 = 5;
const DEGREES: [i32; 5] = [0, 7, 15, 31, 63];
const SEPS: [i32; 5] = [0, 3, 1, 3, 1];
const MINIMUM_STATE_BYTES: usize = 8;
const BREAKS: [usize; 4] = [32, 64, 128, 256];
const DEFAULT_STATE: [i32; 32] = [
    3,
    0x991539b1u32 as i32,
    0x16a5bce3,
    0x6774a4cd,
    0x3e01511e,
    0x4e508aaa,
    0x61048c05,
    0xf5500617u32 as i32,
    0x846b7115u32 as i32,
    0x6a19892c,
    0x896a97afu32 as i32,
    0xdb48f936u32 as i32,
    0x14898454,
    0x37ffd106,
    0xb58bff9cu32 as i32,
    0x59e17104,
    0xcf918a49u32 as i32,
    0x09378c83,
    0x52c7a471,
    0x8d293ea9u32 as i32,
    0x1f4fc301,
    0xc3db71beu32 as i32,
    0x39b44e1c,
    0xf8a44ef9u32 as i32,
    0x4c8b80b1,
    0x19edc328,
    0x87bf4bddu32 as i32,
    0xc9b240e5u32 as i32,
    0xe9ee4b1bu32 as i32,
    0x4382aee7,
    0x535b6b41,
    0xf3bec5dau32 as i32,
];

struct RandomState {
    state: *mut i32,
    front: *mut i32,
    rear: *mut i32,
    end: *mut i32,
    kind: i32,
    degree: i32,
    separation: i32,
}

unsafe impl Send for RandomState {}

static RANDOM: Mutex<Option<RandomState>> = Mutex::new(None);

fn initial() -> RandomState {
    let base = Box::leak(Box::new(DEFAULT_STATE)).as_mut_ptr();
    unsafe {
        RandomState {
            state: base.add(1),
            front: base.add(4),
            rear: base.add(1),
            end: base.add(32),
            kind: 3,
            degree: 31,
            separation: 3,
        }
    }
}

impl RandomState {
    fn next(&mut self) -> i32 {
        // cite: bionic random.c#random_unlocked
        unsafe {
            if self.kind == TYPE_0 {
                let value =
                    (*self.state).wrapping_mul(1103515245).wrapping_add(12345) & 0x7fff_ffff;
                *self.state = value;
                return value;
            }
            *self.front = (*self.front).wrapping_add(*self.rear);
            let value = ((*self.front as u32) >> 1) as i32 & 0x7fff_ffff;
            self.front = self.front.add(1);
            if self.front >= self.end {
                self.front = self.state;
                self.rear = self.rear.add(1);
            } else {
                self.rear = self.rear.add(1);
                if self.rear >= self.end {
                    self.rear = self.state;
                }
            }
            value
        }
    }

    fn seed(&mut self, seed: u32) {
        // cite: bionic random.c#srandom_unlocked
        unsafe {
            *self.state = seed as i32;
            if self.kind == TYPE_0 {
                return;
            }
            for index in 1..self.degree as usize {
                let previous = *self.state.add(index - 1);
                let (high, low) = (previous / 127773, previous % 127773);
                let mut next = 16807i32
                    .wrapping_mul(low)
                    .wrapping_sub(2836i32.wrapping_mul(high));
                if next <= 0 {
                    next = next.wrapping_add(0x7fff_ffff);
                }
                *self.state.add(index) = next;
            }
            self.front = self.state.add(self.separation as usize);
            self.rear = self.state;
            for _ in 0..10 * self.degree {
                self.next();
            }
        }
    }

    fn save_header(&mut self) {
        unsafe {
            let marker = if self.kind == TYPE_0 {
                self.kind
            } else {
                MAX_TYPES * self.rear.offset_from(self.state) as i32 + self.kind
            };
            *self.state.sub(1) = marker;
        }
    }
}

fn with_state<T>(work: impl FnOnce(&mut RandomState) -> T) -> T {
    let mut guard = RANDOM.lock().expect("random state");
    work(guard.get_or_insert_with(initial))
}

pub fn random() -> i32 {
    with_state(RandomState::next)
}

pub fn seed_random(seed: u32) {
    with_state(|state| state.seed(seed));
}

fn initstate(seed: u32, buffer: usize, length: usize) -> usize {
    // cite: bionic random.c#initstate
    with_state(|state| {
        let previous = unsafe { state.state.sub(1) } as usize;
        state.save_header();
        if length < MINIMUM_STATE_BYTES {
            return 0;
        }
        let kind = BREAKS
            .iter()
            .position(|limit| length < *limit)
            .map_or(4, |index| index) as i32;
        state.kind = kind;
        state.degree = DEGREES[kind as usize];
        state.separation = SEPS[kind as usize];
        let words = buffer as *mut i32;
        unsafe {
            state.state = words.add(1);
            state.end = state.state.add(state.degree as usize);
        }
        state.seed(seed);
        let marker = if state.kind == TYPE_0 {
            state.kind
        } else {
            MAX_TYPES * unsafe { state.rear.offset_from(state.state) } as i32 + state.kind
        };
        write_value::<i32>(buffer, marker);
        previous
    })
}

fn setstate(buffer: usize) -> usize {
    // cite: bionic random.c#setstate
    with_state(|state| {
        let first = read_value::<i32>(buffer);
        let (kind, rear) = (first % MAX_TYPES, first / MAX_TYPES);
        let previous = unsafe { state.state.sub(1) } as usize;
        state.save_header();
        if !(0..MAX_TYPES).contains(&kind) {
            return 0;
        }
        state.kind = kind;
        state.degree = DEGREES[kind as usize];
        state.separation = SEPS[kind as usize];
        unsafe {
            state.state = (buffer as *mut i32).add(1);
            if kind != TYPE_0 {
                state.rear = state.state.offset(rear as isize);
                state.front = state
                    .state
                    .offset(((rear + state.separation) % state.degree) as isize);
            }
            state.end = state.state.add(state.degree as usize);
        }
        previous
    })
}

struct Rand48 {
    seed: [u16; 3],
    multiplier: [u16; 3],
    addend: u16,
}

const RAND48_INITIAL: Rand48 = Rand48 {
    seed: [0x330e, 0xabcd, 0x1234],
    multiplier: [0xe66d, 0xdeec, 0x0005],
    addend: 0xb,
};

static RAND48: Mutex<Rand48> = Mutex::new(RAND48_INITIAL);
static SEED48_RESULT: Mutex<[u16; 3]> = Mutex::new([0; 3]);

fn step48(seed: &mut [u16; 3], multiplier: [u16; 3], addend: u16) {
    // cite: bionic _rand48.c#__dorand48
    let mut accumulator = u64::from(multiplier[0]) * u64::from(seed[0]) + u64::from(addend);
    let low = accumulator as u16;
    accumulator >>= 16;
    accumulator += u64::from(multiplier[0]) * u64::from(seed[1])
        + u64::from(multiplier[1]) * u64::from(seed[0]);
    let middle = accumulator as u16;
    accumulator >>= 16;
    accumulator += u64::from(multiplier[0]) * u64::from(seed[2])
        + u64::from(multiplier[1]) * u64::from(seed[1])
        + u64::from(multiplier[2]) * u64::from(seed[0]);
    *seed = [low, middle, accumulator as u16];
}

fn step_global() -> [u16; 3] {
    let mut generator = RAND48.lock().expect("rand48");
    let (multiplier, addend) = (generator.multiplier, generator.addend);
    step48(&mut generator.seed, multiplier, addend);
    generator.seed
}

fn step_caller(address: usize) -> [u16; 3] {
    let generator = RAND48.lock().expect("rand48");
    let mut seed = [
        read_value::<u16>(address),
        read_value::<u16>(address + 2),
        read_value::<u16>(address + 4),
    ];
    step48(&mut seed, generator.multiplier, generator.addend);
    for (index, word) in seed.iter().enumerate() {
        write_value::<u16>(address + index * 2, *word);
    }
    seed
}

fn as_long(seed: [u16; 3]) -> i64 {
    // cite: bionic lrand48.c#lrand48
    i64::from(seed[2]) * 32768 + i64::from(seed[1] >> 1)
}

fn as_signed_long(seed: [u16; 3]) -> i64 {
    // cite: bionic mrand48.c#mrand48
    i64::from(seed[2] as i16) * 65536 + i64::from(seed[1])
}

fn as_double(seed: [u16; 3]) -> f64 {
    // cite: bionic erand48.c#erand48
    libm_ldexp(f64::from(seed[0]), -48)
        + libm_ldexp(f64::from(seed[1]), -32)
        + libm_ldexp(f64::from(seed[2]), -16)
}

fn libm_ldexp(value: f64, exponent: i32) -> f64 {
    value * 2f64.powi(exponent)
}

fn seed_global(seed: i64) {
    // cite: bionic srand48.c#srand48
    *RAND48.lock().expect("rand48") = Rand48 {
        seed: [0x330e, seed as u16, ((seed as u64) >> 16) as u16],
        ..RAND48_INITIAL
    };
}

pub fn register() {
    in_libc("rand", |context| context.ret_i32(random()));
    in_libc("random", |context| context.ret_i64(i64::from(random())));
    in_libc("srand", |context| seed_random(context.arg_u32(0)));
    in_libc("srandom", |context| seed_random(context.arg_u32(0)));
    in_libc("initstate", |context| {
        context.ret_ptr(initstate(
            context.arg_u32(0),
            context.arg_ptr(1),
            context.arg_u64(2) as usize,
        ))
    });
    in_libc("setstate", |context| {
        context.ret_ptr(setstate(context.arg_ptr(0)))
    });
    in_libc("rand_r", |context| {
        // cite: bionic rand_r.c#rand_r
        let address = context.arg_ptr(0);
        let next = read_value::<u32>(address)
            .wrapping_mul(1103515245)
            .wrapping_add(12345);
        write_value::<u32>(address, next);
        context.ret_i32((next & 0x7fff_ffff) as i32);
    });
    in_libc("srand48", |context| seed_global(context.arg_u64(0) as i64));
    in_libc("lrand48", |context| context.ret_i64(as_long(step_global())));
    in_libc("mrand48", |context| {
        context.ret_i64(as_signed_long(step_global()))
    });
    in_libc("drand48", |context| {
        context.ret_f64(as_double(step_global()))
    });
    in_libc("nrand48", |context| {
        context.ret_i64(as_long(step_caller(context.arg_ptr(0))))
    });
    in_libc("jrand48", |context| {
        context.ret_i64(as_signed_long(step_caller(context.arg_ptr(0))))
    });
    in_libc("erand48", |context| {
        context.ret_f64(as_double(step_caller(context.arg_ptr(0))))
    });
    in_libc("seed48", |context| {
        // cite: bionic seed48.c#seed48
        let address = context.arg_ptr(0);
        let mut generator = RAND48.lock().expect("rand48");
        let mut previous = SEED48_RESULT.lock().expect("seed48 result");
        *previous = generator.seed;
        *generator = Rand48 {
            seed: [
                read_value(address),
                read_value(address + 2),
                read_value(address + 4),
            ],
            ..RAND48_INITIAL
        };
        context.ret_ptr(previous.as_ptr() as usize);
    });
    in_libc("lcong48", |context| {
        // cite: bionic lcong48.c#lcong48
        let address = context.arg_ptr(0);
        let word = |index: usize| read_value::<u16>(address + index * 2);
        *RAND48.lock().expect("rand48") = Rand48 {
            seed: [word(0), word(1), word(2)],
            multiplier: [word(3), word(4), word(5)],
            addend: word(6),
        };
    });
}
