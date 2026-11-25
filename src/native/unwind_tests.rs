use std::sync::Mutex;

use super::bionic::guest_mem::read_value;
use super::bridge_test_support::register_all;
use super::call_assign::CallBuilder;
use super::dl_api::DlHandle;
use super::unwind::fde::{self, Cie, Fde, Memory};
use super::loader_test_support::{child_outcome, dlopen_without_constructors, fixture_dir};
use super::unwind::registers::{UnwindContext, UnwindRegs};
use super::runtime::NativeRuntime;
use super::thread_test_support::shim;
use super::unwind::cfi::{self, CfaRule, Rule};
use super::unwind::walk::{self, Space};
use super::unwind::CONTEXT_MAGIC;

static CASE_LOCK: Mutex<()> = Mutex::new(());

struct Program {
    bytes: Vec<u8>,
    cie: (usize, usize),
    fde: (usize, usize),
}

impl Program {
    fn new(cie: &[u8], fde: &[u8]) -> Self {
        let mut bytes = Vec::new();
        let cie_start = bytes.len();
        bytes.extend_from_slice(cie);
        let cie_end = bytes.len();
        let fde_start = bytes.len();
        bytes.extend_from_slice(fde);
        let fde_end = bytes.len();
        Self { bytes, cie: (cie_start, cie_end), fde: (fde_start, fde_end) }
    }

    fn memory(&self) -> Memory {
        let start = self.bytes.as_ptr() as usize;
        Memory::block(start, start + self.bytes.len())
    }

    fn description(&self) -> Fde {
        let start = self.bytes.as_ptr() as usize;
        Fde {
            address: start + self.fde.0,
            initial_location: 0x1000,
            range: 0x400,
            lsda: 0,
            cie: Cie {
                code_align: 1,
                data_align: -8,
                return_register: 30,
                pointer_encoding: 0x00,
                lsda_encoding: 0,
                personality: 0,
                has_z: false,
                instructions: (start + self.cie.0, start + self.cie.1),
            },
            instructions: (start + self.fde.0, start + self.fde.1),
        }
    }

    fn rules(&self, pc: u64) -> cfi::RuleSet {
        let memory = self.memory();
        cfi::execute(&memory, &self.description(), pc).unwrap_or_else(|error| panic!("{error}"))
    }

    fn error(&self, pc: u64) -> String {
        let memory = self.memory();
        cfi::execute(&memory, &self.description(), pc).unwrap_err()
    }
}

fn cfa_offset(rules: &cfi::RuleSet) -> Option<(u16, i64)> {
    match rules.cfa {
        Some(CfaRule::Register(register, offset)) => Some((register, offset)),
        _ => None,
    }
}

#[test]
fn dwarf_cfi_cie_initial_instructions_set_the_cfa_and_the_return_address_rule_for_every_row() {
    let program = Program::new(&[0x0c, 31, 0, 0x05, 30, 1], &[0x00]);
    let rules = program.rules(0x1000);

    assert_eq!(cfa_offset(&rules), Some((31, 0)));
    assert_eq!(rules.rule(30), Rule::Offset(1));
    assert_eq!(rules.rule(0), Rule::Undefined);
}

#[test]
fn dwarf_cfi_advance_loc_rows_apply_only_from_their_location_onward() {
    let program = Program::new(&[0x0c, 31, 0], &[0x48, 0x0e, 16, 0x48, 0x0e, 32]);

    assert_eq!(cfa_offset(&program.rules(0x1000)), Some((31, 0)));
    assert_eq!(cfa_offset(&program.rules(0x1008)), Some((31, 16)));
    assert_eq!(cfa_offset(&program.rules(0x1010)), Some((31, 32)));
}

#[test]
fn dwarf_cfi_remember_and_restore_state_undo_a_row_between_two_locations() {
    let program = Program::new(&[0x0c, 31, 0], &[0x0e, 16, 0x0a, 0x0e, 32, 0x0b, 0x48]);

    assert_eq!(cfa_offset(&program.rules(0x1000)), Some((31, 16)));
    assert_eq!(cfa_offset(&program.rules(0x1008)), Some((31, 16)));
}

#[test]
fn dwarf_cfi_restore_and_restore_extended_return_to_the_cie_rule() {
    let program = Program::new(&[0x0c, 31, 0, 0x05, 30, 1], &[0x05, 30, 2, 0xc0 | 30, 0x05, 5, 7, 0x06, 5]);

    let rules = program.rules(0x1000);
    assert_eq!(rules.rule(30), Rule::Offset(1), "restore returns to the CIE rule");
    assert_eq!(rules.rule(5), Rule::Undefined, "restore_extended returns to the CIE rule");
}

#[test]
fn dwarf_cfi_register_undefined_same_value_and_val_offset_rules_are_recorded_verbatim() {
    let program = Program::new(&[0x0c, 31, 0], &[0x09, 7, 31, 0x07, 8, 0x08, 9, 0x14, 5, 3, 0x15, 6, 0x7e]);

    let rules = program.rules(0x1000);
    assert_eq!(rules.rule(7), Rule::Register(31));
    assert_eq!(rules.rule(8), Rule::Undefined);
    assert_eq!(rules.rule(9), Rule::SameValue);
    assert_eq!(rules.rule(5), Rule::ValueOffset(3));
    assert_eq!(rules.rule(6), Rule::ValueOffset(-2));
}

#[test]
fn dwarf_cfi_expression_forms_carry_the_expression_bytes_and_a_def_cfa_expression_replaces_the_cfa() {
    let program = Program::new(&[0x0c, 31, 0, 0x05, 30, 1], &[0x10, 5, 1, 0x30, 0x16, 6, 2, 0x30, 0x9f]);

    let rules = program.rules(0x1000);
    match rules.rule(5) {
        Rule::Expression(start, length) => {
            assert_eq!(length, 1);
            assert_eq!(unsafe { std::slice::from_raw_parts(start as *const u8, length) }, [0x30]);
        }
        other => panic!("rule 5 is {other:?}"),
    }
    match rules.rule(6) {
        Rule::ValueExpression(_, length) => assert_eq!(length, 2),
        other => panic!("rule 6 is {other:?}"),
    }

    let expression = Program::new(&[0x0c, 31, 0, 0x05, 30, 1], &[0x0f, 2, 0x30, 0x9f]);
    match expression.rules(0x1000).cfa {
        Some(CfaRule::Expression(_, length)) => assert_eq!(length, 2),
        other => panic!("the CFA rule is {other:?}"),
    }
}

#[test]
fn dwarf_cfi_gnu_args_size_is_parsed_and_ignored_and_the_negative_offset_form_negates_the_factor() {
    let program = Program::new(&[0x0c, 31, 0], &[0x2e, 7, 0x0e, 32, 0x2f, 9, 4]);

    let rules = program.rules(0x1000);
    assert_eq!(cfa_offset(&rules), Some((31, 32)));
    assert_eq!(rules.rule(9), Rule::Offset(-4));
}

#[test]
fn dwarf_cfi_set_loc_moves_the_row_location_to_an_absolute_address() {
    let program = Program::new(&[0x0c, 31, 0], &[0x0e, 16, 0x01, 0x20, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 64]);

    assert_eq!(cfa_offset(&program.rules(0x1018)), Some((31, 16)));
    assert_eq!(cfa_offset(&program.rules(0x1020)), Some((31, 64)));
}

#[test]
fn dwarf_cfi_unassigned_opcodes_and_unknown_registers_are_refused_with_the_opcode_in_the_message() {
    let opcode = Program::new(&[0x0c, 31, 0], &[0x2d]).error(0x1000);
    assert!(opcode.contains("DWARF CFI opcode 0x2d"), "{opcode}");
    assert!(opcode.contains("next missing primitive"), "{opcode}");

    let register = Program::new(&[0x0c, 31, 0], &[0x05, 0xc8, 0x01, 0x01]).error(0x1000);
    assert!(register.contains("DWARF register 200"), "{register}");
}

#[test]
fn eh_frame_pointer_encodings_decode_every_supported_format_and_refuse_the_unsupported_applications() {
    let mut data = vec![0u8; 64];
    let base = data.as_ptr() as usize;
    data[0..8].copy_from_slice(&0x1234_5678_9abc_def0u64.to_le_bytes());
    data[8..12].copy_from_slice(&7u32.to_le_bytes());
    data[12..14].copy_from_slice(&9u16.to_le_bytes());
    data[16] = 0x81;
    data[17] = 0x01;
    data[18..22].copy_from_slice(&(-2i32).to_le_bytes());
    data[24..32].copy_from_slice(&((base + 40) as u64).to_le_bytes());
    data[40..48].copy_from_slice(&0x99u64.to_le_bytes());
    let memory = Memory::block(base, base + data.len());

    let mut at = base;
    assert_eq!(memory.encoded(&mut at, 0x00, 0).unwrap(), 0x1234_5678_9abc_def0);
    assert_eq!(at, base + 8);
    assert_eq!(memory.encoded(&mut at, 0x03, 0).unwrap(), 7);
    assert_eq!(memory.encoded(&mut at, 0x0b, 0).unwrap(), 9);
    assert_eq!(memory.encoded(&mut at, 0x02, 0).unwrap(), 0x181);
    let mut at = base + 16;
    assert_eq!(memory.encoded(&mut at, 0x01, 0).unwrap(), 129);
    assert_eq!(memory.encoded(&mut at, 0x0b, 0).unwrap(), u64::MAX - 1, "sdata4 signs extends to the host width");

    let mut at = base + 8;
    assert_eq!(memory.encoded(&mut at, 0x33, base as u64 + 0x1000).unwrap(), base as u64 + 0x1007, "datarel adds the section base");
    let mut at = base + 8;
    assert_eq!(memory.encoded(&mut at, 0x13, 0).unwrap(), base as u64 + 8 + 7, "pcrel adds the field address");
    let mut at = base + 24;
    assert_eq!(memory.encoded(&mut at, 0x80, 0).unwrap(), 0x99, "indirect dereferences the decoded pointer");

    for encoding in [0x20u8, 0x40, 0x50] {
        let mut at = base;
        let error = memory.encoded(&mut at, encoding, 0).unwrap_err();
        assert!(error.contains("application 0x") && error.contains("next missing primitive"), "{encoding:#x}: {error}");
    }
    let mut at = base;
    assert!(memory.encoded(&mut at, 0x0f, 0).unwrap_err().contains("pointer format 0xf"));
}

fn running_space() -> Space {
    Space { stack_low: 0x1000_0000, stack_high: 0x1001_0000 }
}

fn probe_frame() -> fde::Found {
    fde::Found {
        memory: Memory::block(0x2000_0000, 0x2000_0100),
        fde: Fde {
            address: 0x2000_0000,
            initial_location: 0x1000,
            range: 16,
            lsda: 0,
            cie: Cie {
                code_align: 1,
                data_align: -8,
                return_register: 30,
                pointer_encoding: 0,
                lsda_encoding: 0,
                personality: 0,
                has_z: false,
                instructions: (0x2000_0000, 0x2000_0000),
            },
            instructions: (0x2000_0000, 0x2000_0000),
        },
        name: "probe".to_string(),
    }
}

#[test]
fn unwinder_guards_refuse_a_cfa_below_the_stack_pointer_or_misaligned_or_outside_the_thread_stack() {
    let space = running_space();
    let found = probe_frame();
    let mut regs = UnwindRegs::zeroed();
    regs.sp = 0x1000_0100;

    let below = walk::guard(&space, &regs, 0x1000_00f0, &found).unwrap_err();
    assert!(below.contains("unwind stack direction"), "{below}");
    let misaligned = walk::guard(&space, &regs, 0x1000_0108, &found).unwrap_err();
    assert!(misaligned.contains("unwind stack alignment"), "{misaligned}");
    let outside = walk::guard(&space, &regs, 0x2000_0000, &found).unwrap_err();
    assert!(outside.contains("unwind stack bounds"), "{outside}");
    assert!(walk::guard(&space, &regs, 0x1000_0120, &found).is_ok());
}

#[test]
fn itanium_cxx_abi_accessors_refuse_a_context_this_unwinder_did_not_create() {
    register_all();
    let (signal, output) = child_outcome(|| {
        let mut bogus = [0u64; 64];
        let _ = shim("_Unwind_GetCFA").ptr(bogus.as_mut_ptr() as usize).invoke();
    });

    assert_eq!(signal, Some(libc::SIGABRT), "{output}");
    assert!(output.contains("cross-unwinder _Unwind_Context"), "{output}");
}

#[test]
fn itanium_cxx_abi_accessors_read_and_write_the_general_vector_and_control_registers_of_a_context() {
    register_all();
    let mut context = Box::new(UnwindContext::new(UnwindRegs::zeroed()));
    context.regs.x[0] = 0x11;
    context.regs.x[30] = 0x22;
    context.regs.sp = 0x3000;
    context.regs.pc = 0x4000;
    context.regs.q[0] = 0x5555;
    context.cfa = 0x5000;
    context.region_start = 0x6000;
    context.lsda = 0x7000;
    let pointer = (&mut *context as *mut UnwindContext) as usize;

    let value = |name: &str, index: i32| shim(name).ptr(pointer).i32(index).invoke().unwrap().u64();
    assert_eq!(value("_Unwind_GetGR", 0), 0x11);
    assert_eq!(value("_Unwind_GetGR", 30), 0x22);
    assert_eq!(value("_Unwind_GetGR", 31), 0x3000);
    assert_eq!(value("_Unwind_GetGR", 72), 0x5555);
    assert_eq!(value("_Unwind_GetIP", 0), 0x4000);
    assert_eq!(value("_Unwind_GetCFA", 0), 0x5000);
    assert_eq!(value("_Unwind_GetRegionStart", 0), 0x6000);
    assert_eq!(value("_Unwind_GetLanguageSpecificData", 0), 0x7000);
    assert_eq!(value("_Unwind_GetTextRelBase", 0), 0);
    assert_eq!(value("_Unwind_GetDataRelBase", 0), 0);

    shim("_Unwind_SetGR").ptr(pointer).i32(3).int(0x33).invoke().unwrap();
    shim("_Unwind_SetIP").ptr(pointer).int(0x4400).invoke().unwrap();
    assert_eq!(context.regs.x[3], 0x33);
    assert_eq!(context.regs.pc, 0x4400);
    assert_eq!(context.magic, CONTEXT_MAGIC);
}

fn eh_frame_block(initial: u64, range: u64) -> (Vec<u8>, usize) {
    let mut bytes = Vec::new();
    let mut cie = Vec::new();
    cie.push(1);
    cie.extend_from_slice(b"zR\0");
    cie.push(1);
    cie.push(0x78);
    cie.push(30);
    cie.push(1);
    cie.push(0x00);
    cie.extend_from_slice(&[0x0c, 31, 0]);
    bytes.extend_from_slice(&((cie.len() + 4) as u32).to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&cie);

    let fde_at = bytes.len();
    let mut fde = Vec::new();
    fde.extend_from_slice(&((fde_at + 4) as u32).to_le_bytes());
    fde.extend_from_slice(&initial.to_le_bytes());
    fde.extend_from_slice(&range.to_le_bytes());
    fde.push(0);
    fde.push(0x00);
    bytes.extend_from_slice(&((fde.len() + 4) as u32).to_le_bytes());
    bytes.extend_from_slice(&fde);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    (bytes, fde_at)
}

#[test]
fn register_frame_makes_a_hand_built_fde_visible_to_find_fde_and_deregister_frame_removes_it() {
    register_all();
    let (block, fde_at) = eh_frame_block(0x7000_0000, 0x40);
    let begin = block.as_ptr() as usize;
    let mut bases = [0u64; 3];
    assert_eq!(shim("_Unwind_FindEnclosingFunction").int(0x7000_0010).invoke().unwrap().u64(), 0);

    fde::register_frame(begin);
    let located = shim("_Unwind_Find_FDE").int(0x7000_0010).ptr(bases.as_mut_ptr() as usize).invoke().unwrap().u64();
    assert_eq!(located as usize, begin + fde_at);
    assert_eq!(bases, [0x7000_0000, 0, 0]);
    assert_eq!(shim("_Unwind_FindEnclosingFunction").int(0x7000_0010).invoke().unwrap().u64(), 0x7000_0000);
    assert_eq!(shim("_Unwind_FindEnclosingFunction").int(0x7000_0050).invoke().unwrap().u64(), 0, "past the FDE range");

    fde::deregister_frame(begin);
    assert_eq!(shim("_Unwind_Find_FDE").int(0x7000_0010).ptr(bases.as_mut_ptr() as usize).invoke().unwrap().u64(), 0);
}

fn unwind_runtime() -> &'static NativeRuntime {
    register_all();
    NativeRuntime::initialize(vec![fixture_dir("16k")]).expect("the native runtime is initialized with the 16k fixture directory")
}

fn unwind_handle() -> u64 {
    dlopen_without_constructors(unwind_runtime().loader(), "libfx_unwind.so").unwrap_or_else(|error| panic!("libfx_unwind.so: {error}"))
}

fn symbol(handle: u64, name: &str) -> usize {
    unwind_runtime().loader().dlsym(DlHandle::Module(handle), name, None).unwrap_or_else(|error| panic!("{name}: {error}"))
}

#[test]
fn bionic_every_exported_function_of_the_unwind_fixture_has_an_fde_and_only_the_cleanup_frames_carry_a_personality() {
    let handle = unwind_handle();
    let loader = unwind_runtime().loader();
    let info = loader.module_info(handle).unwrap();
    let plain = symbol(handle, "fx_unwind_backtrace_driver");
    let reporting = symbol(handle, "fx_unwind_outer_caller");

    let plain_fde = fde::lookup(plain as u64).unwrap().expect("the plain fixture function has an FDE");
    assert_eq!(plain_fde.fde.initial_location, plain as u64);
    assert_eq!(plain_fde.fde.cie.personality, 0);
    assert_eq!(plain_fde.fde.lsda, 0, "a CIE without a personality has no LSDA");

    let reporting_fde = fde::lookup(reporting as u64).unwrap().expect("the cleanup fixture function has an FDE");
    assert_eq!(reporting_fde.fde.initial_location, reporting as u64);
    assert_ne!(reporting_fde.fde.cie.personality, 0, "the zPLR CIE names __gcc_personality_v0");
    assert_ne!(reporting_fde.fde.lsda, 0);
    assert!(
        reporting_fde.fde.lsda >= info.region as u64 && reporting_fde.fde.lsda < (info.region + info.region_len) as u64,
        "the LSDA lies inside the mapped image"
    );
    assert_eq!(fde::find_enclosing(reporting as u64 + 4), Some(reporting as u64));
    assert_eq!(fde::find_enclosing(0), None);
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Frame {
    ip: usize,
    cfa: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Observation {
    ip: usize,
    region: usize,
    lsda: usize,
    resume_ip: usize,
    lsda_error: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Hit {
    region: usize,
    count: i32,
}

fn call_case(handle: u64, name: &str) -> i32 {
    CallBuilder::new(symbol(handle, name)).invoke().unwrap().i32()
}

fn call_i32(handle: u64, name: &str) -> i32 {
    CallBuilder::new(symbol(handle, name)).invoke().unwrap().i32()
}

#[test]
fn itanium_cxx_abi_raise_installs_a_handler_in_the_frame_that_called_it() {
    let _guard = CASE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let handle = unwind_handle();
    CallBuilder::new(symbol(handle, "fx_unwind_reset")).invoke().unwrap();

    let result = call_case(handle, "fx_unwind_case_thrower");

    assert_eq!(result, 41, "the installed handler resumed the thrower with its own return value");
    assert_eq!(call_i32(handle, "fx_unwind_catch_runs"), 1);
    assert_eq!(call_i32(handle, "fx_unwind_handler_found"), 1);
    let caught = CallBuilder::new(symbol(handle, "fx_unwind_catch_exception")).invoke().unwrap().u64();
    let expected = CallBuilder::new(symbol(handle, "fx_unwind_case_exception")).i32(0).invoke().unwrap().u64();
    assert_eq!(caught, expected, "x0 held the exception object the shim was given");
    let class = read_value::<u64>(caught as usize);
    assert_eq!(class, 0x5744_554e_5749_4e44, "the exception class round tripped");
}

#[test]
fn itanium_cxx_abi_raise_steps_to_a_handler_two_frames_up_and_restores_its_stack_pointer() {
    let _guard = CASE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let handle = unwind_handle();
    CallBuilder::new(symbol(handle, "fx_unwind_reset")).invoke().unwrap();

    let result = call_case(handle, "fx_unwind_case_caller");

    assert_eq!(result, 41, "the handler ran in the outer frame and the walk skipped both plain frames");
    assert_eq!(call_i32(handle, "fx_unwind_handler_found"), 1);
    let regions = symbol(handle, "fx_unwind_case_caller");
    assert_ne!(regions, 0);
    let mut observation = Observation::default();
    CallBuilder::new(symbol(handle, "fx_unwind_observe")).ptr((&mut observation as *mut Observation) as usize).invoke().unwrap();
    assert_eq!(observation.region, symbol(handle, "fx_unwind_outer_caller"), "the handler frame is the outer function");
    assert!(
        observation.ip <= observation.resume_ip && observation.resume_ip - observation.ip < 64,
        "the observed IP is the call site: ip {:#x} resume {:#x} region {:#x} lp {:#x}",
        observation.ip,
        observation.resume_ip,
        observation.region,
        symbol(handle, "fx_unwind_outer_caller")
    );
    assert_ne!(observation.lsda, 0);
    assert_eq!(observation.lsda_error, 0);
}

#[test]
fn itanium_cxx_abi_lsda_call_site_table_drives_a_cleanup_that_resumes_into_the_handler() {
    let _guard = CASE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let handle = unwind_handle();
    CallBuilder::new(symbol(handle, "fx_unwind_reset")).invoke().unwrap();

    let result = call_case(handle, "fx_unwind_case_cleanup");

    assert_eq!(result, 41);
    assert!(call_i32(handle, "fx_unwind_cleanup_runs") >= 1, "the landing pad ran the cleanup");
    let cleaned = CallBuilder::new(symbol(handle, "fx_unwind_cleanup_exception")).invoke().unwrap().u64();
    let caught = CallBuilder::new(symbol(handle, "fx_unwind_catch_exception")).invoke().unwrap().u64();
    let expected = CallBuilder::new(symbol(handle, "fx_unwind_case_exception")).i32(2).invoke().unwrap().u64();
    assert_eq!(cleaned, expected, "the cleanup saw the exception object in x0");
    assert_eq!(caught, expected, "the resumed exception carried the same object");
}

#[test]
fn itanium_cxx_abi_backtrace_reports_monotonic_aligned_frames_that_map_to_their_enclosing_functions() {
    let _guard = CASE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let handle = unwind_handle();
    let mut frames = vec![Frame::default(); 16];
    let count = CallBuilder::new(symbol(handle, "fx_unwind_backtrace_driver"))
        .ptr(frames.as_mut_ptr() as usize)
        .i32(16)
        .invoke()
        .unwrap()
        .i32();
    assert_eq!(count, 2, "the walk ends at the host call boundary, so only the two fixture frames are visible");

    for pair in frames[..count as usize].windows(2) {
        assert!(pair[0].cfa <= pair[1].cfa, "the CFA must not move down the stack");
        assert_eq!(pair[1].cfa % 16, 0, "every CFA is 16 byte aligned");
    }
    let enclosing = |ip: usize| CallBuilder::new(symbol(handle, "fx_unwind_enclosing")).int(ip as u64).invoke().unwrap().u64();
    assert_eq!(enclosing(frames[0].ip), symbol(handle, "fx_unwind_backtrace") as u64);
    assert_eq!(enclosing(frames[1].ip), symbol(handle, "fx_unwind_backtrace_driver") as u64);
}

#[test]
fn itanium_cxx_abi_frames_without_a_personality_are_skipped_by_the_personality_walk() {
    let _guard = CASE_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let handle = unwind_handle();
    CallBuilder::new(symbol(handle, "fx_unwind_reset")).invoke().unwrap();
    call_case(handle, "fx_unwind_case_caller");
    let mut hits = vec![Hit::default(); 16];
    let total = CallBuilder::new(symbol(handle, "fx_unwind_hits")).ptr(hits.as_mut_ptr() as usize).i32(16).invoke().unwrap().i32();
    assert!(total >= 1);
    let count_for = |region: usize| hits[..total as usize].iter().find(|hit| hit.region == region).map_or(0, |hit| hit.count);

    assert_eq!(count_for(symbol(handle, "fx_unwind_outer_caller")), 2, "the handler frame is examined in both phases");
    assert_eq!(count_for(symbol(handle, "fx_unwind_thrower_plain")), 0, "a frame without a personality never calls one");
    assert_eq!(count_for(symbol(handle, "fx_unwind_middle_plain")), 0);
}
