use super::bridge_test_support::Bridged;
use super::elf_image::ElfImage;
use super::guest_thread::{art_thread_slot_is_null, attach_current};
use super::image_layout::{ImageLayout, HOST_PAGE};
use super::image_map::MappedImage;
use super::loader_test_support::fixture_dir;
use super::thread_block::{block, guest_thread_pointer, host_thread_pointer, slot_word};
use super::tp_veneer::{redirect_site, veneer_words};

pub fn veneer_for(address: usize, word: u32) -> Option<[u32; 4]> {
    if word & 0xFC00_0000 != 0x1400_0000 {
        return None;
    }
    let displacement = (((word & 0x03FF_FFFF) << 6) as i32 >> 6) as i64 * 4;
    let veneer = (address as i64 + displacement) as usize;
    let words: [u32; 4] = std::array::from_fn(|index| unsafe { *(veneer as *const u32).add(index) });
    (words[0] & 0xFFFF_FFE0 == 0xD53B_D060).then_some(words)
}

fn host_slot(index: usize) -> u64 {
    unsafe { *(host_thread_pointer() as *const u64).add(index) }
}

#[test]
fn thread_pointer_veneer_for_x5_encodes_mrs_two_adds_of_the_bias_and_a_branch_back() {
    let words = veneer_words(5, 0x1000, 0x2000, 0x1238).unwrap();

    assert_eq!(words[0], 0xD53B_D065);
    assert_eq!(words[1], 0x9108_E0A5);
    assert_eq!(words[2], 0x9140_04A5);
    assert_eq!(words[3], 0x1400_0000 | ((0x1004u32.wrapping_sub(0x200c) >> 2) & 0x03FF_FFFF));
}

#[test]
fn thread_pointer_veneer_refuses_a_site_more_than_128_mib_away() {
    let error = veneer_words(1, 0x1000, 0x1000 + (128 << 20) + 16, 8).unwrap_err().to_string();

    assert!(error.contains("out of reach"), "{error}");
    assert!(veneer_words(1, 0x1000, 0x1000 + (128 << 20) - 64, 8).is_ok());
}

#[test]
fn a_thread_pointer_read_into_xzr_becomes_a_nop_and_an_exhausted_veneer_area_is_an_explicit_error() {
    let data = std::fs::read(fixture_dir("16k").join("libfx_add.so")).unwrap();
    let elf = ElfImage::parse("libfx_add.so", &data).unwrap();
    let image = MappedImage::map("libfx_add.so", &data, &elf).unwrap();

    assert_eq!(redirect_site(&image, image.region(), 31, 8).unwrap(), 0xD503_201F);
    let error = redirect_site(&image, image.region(), 3, 8).unwrap_err().to_string();
    assert!(error.contains("veneer area is exhausted"), "{error}");
}

#[test]
fn a_mapped_image_reserves_one_veneer_per_thread_pointer_read_in_executable_segments_after_the_image() {
    let data = std::fs::read(fixture_dir("16k").join("libfx_reloc.so")).unwrap();
    let elf = ElfImage::parse("libfx_reloc.so", &data).unwrap();
    let plain = ImageLayout::plan(&elf, HOST_PAGE).unwrap();
    let image = MappedImage::map("libfx_reloc.so", &data, &elf).unwrap();
    let layout = *image.layout();

    assert!(layout.veneer_len > 0 && layout.veneer_len % 16 == 0);
    assert_eq!(layout.veneer_start, plain.region_len);
    assert_eq!(layout.region_len, plain.region_len + layout.veneer_len.div_ceil(HOST_PAGE) * HOST_PAGE);
    let ranges = layout.page_protections(&elf).unwrap();
    let tail = ranges.last().unwrap();
    assert_eq!((tail.start, tail.prot), (layout.veneer_start, libc::PROT_READ | libc::PROT_EXEC));
}

#[test]
fn guest_thread_pointer_is_the_host_tsd_base_plus_the_block_bias_and_every_slot_is_bionic_shaped() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let record = attach_current().unwrap() as u64;

    let guest_tp = bridged.call("fx_tp_value").invoke().unwrap().u64() as usize;

    assert_eq!(guest_tp, guest_thread_pointer());
    assert_eq!(guest_tp, host_thread_pointer() + block().bias());
    let slot = |index: i64| bridged.call("fx_tp_slot").int(index as u64).invoke().unwrap().u64();
    assert_eq!(slot(1), record, "TLS_SLOT_THREAD_ID is the pthread_internal_t of the thread");
    assert_eq!(slot(5), block().canary(), "TLS_SLOT_STACK_GUARD");
    assert_ne!(block().canary(), 0);
    let keys = unsafe { (*(record as *const crate::native::pthread_internal::PthreadInternal)).bionic_tls };
    assert_ne!(keys, 0);
    assert_eq!(slot(-1), keys, "TLS_SLOT_BIONIC_TLS points at the key table block");
    for empty in [0i64, 2, 3, 4, 6, 7] {
        assert_eq!(slot(empty), 0, "slot {empty}");
    }
}

#[test]
fn immediate_offset_loads_of_the_thread_id_and_art_thread_slots_read_bionic_values() {
    let bridged = Bridged::load("16k", "libfx_bridge.so");
    let record = attach_current().unwrap() as u64;

    assert_eq!(bridged.call("fx_tp_thread_id").invoke().unwrap().u64(), record);
    assert_eq!(bridged.call("fx_tp_art_thread").invoke().unwrap().u64(), 0);
    assert_eq!(bridged.call("fx_tp_thread_id_after_call").invoke().unwrap().u64(), record, "tp kept in a callee saved register across a call");
}

#[test]
fn guest_stores_to_bionic_slots_stay_per_thread_and_leave_the_macos_tsd_words_alone() {
    let bridged = std::sync::Arc::new(Bridged::load("16k", "libfx_bridge.so"));
    attach_current().unwrap();
    let before: Vec<u64> = (0..10).map(host_slot).collect();

    for (slot, value) in [(2i64, 0x22u64), (3, 0x33), (4, 0x44), (6, 0x66), (7, 0x77)] {
        bridged.call("fx_tp_store").int(slot as u64).int(value).invoke().unwrap();
        assert_eq!(bridged.call("fx_tp_slot").int(slot as u64).invoke().unwrap().u64(), value);
    }

    assert_eq!((0..10).map(host_slot).collect::<Vec<_>>(), before);
    assert_eq!(slot_word(7), 0x77);
    assert!(!art_thread_slot_is_null());
    let other = {
        let bridged = bridged.clone();
        std::thread::spawn(move || (bridged.call("fx_tp_slot").int(7).invoke().unwrap().u64(), bridged.call("fx_tp_slot").int(5).invoke().unwrap().u64()))
            .join()
            .unwrap()
    };
    assert_eq!(other, (0, block().canary()), "a new thread starts with empty application slots and the shared canary");
    bridged.call("fx_tp_store").int(7).int(0).invoke().unwrap();
}
