use crate::error::{Result, WinedroidError};

use super::image_map::MappedImage;

const MRS_TPIDRRO_EL0: u32 = 0xD53B_D060;
const ADD_IMMEDIATE_64: u32 = 0x9100_0000;
const ADD_IMMEDIATE_SHIFTED_64: u32 = 0x9140_0000;
const BRANCH: u32 = 0x1400_0000;
const NOP: u32 = 0xD503_201F;
const BRANCH_REACH: i64 = 128 << 20;
pub const VENEER_BYTES: usize = 16;

fn add(register: u32, immediate: usize, base: u32) -> u32 {
    base | ((immediate as u32 & 0xFFF) << 10) | (register << 5) | register
}

fn branch(from: usize, to: usize) -> Result<u32> {
    let delta = to as i64 - from as i64;
    if delta % 4 != 0 || delta <= -BRANCH_REACH || delta >= BRANCH_REACH {
        return Err(WinedroidError::new(format!(
            "branch from {from:#x} to {to:#x} is out of reach; next missing primitive: thread pointer veneer beyond 128 MiB"
        )));
    }
    Ok(BRANCH | ((delta / 4) as u32 & 0x03FF_FFFF))
}

pub fn veneer_words(register: u32, site: usize, veneer: usize, bias: usize) -> Result<[u32; 4]> {
    Ok([
        MRS_TPIDRRO_EL0 | register,
        add(register, bias, ADD_IMMEDIATE_64),
        add(register, bias >> 12, ADD_IMMEDIATE_SHIFTED_64),
        branch(veneer + 12, site + 4)?,
    ])
}

pub fn redirect_site(image: &MappedImage, site: usize, register: u32, bias: usize) -> Result<u32> {
    if register == 31 {
        return Ok(NOP);
    }
    let veneer = image.reserve_veneer(VENEER_BYTES)?;
    let words = veneer_words(register, site, veneer, bias)?;
    let entry = branch(site, veneer)?;
    unsafe { std::ptr::copy_nonoverlapping(words.as_ptr(), veneer as *mut u32, words.len()) };
    Ok(entry)
}
