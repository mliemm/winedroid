#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod data_fault;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod dl_api;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod dl_lock;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod dynamic_table;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod elf_image;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod guest_call;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod guest_context;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod image_layout;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod image_map;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod handler_registry;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod host_log;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod init_fini;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod library_search;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod link_resolution;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod loader;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod mach_task;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod module;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod quad;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod relocate;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod reloc_stream;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod symbol_table;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod system_libraries;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod text_patch;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod unload;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod tls_modules;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod bionic;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod bridge;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod call_assign;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod guest_regs;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod fork_gate;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod futex;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod futex_syscall;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod guest_thread;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod pthread_internal;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod runtime;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod signal_frame;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod signal_numbers;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod signal_router;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod signal_state;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod stub_table;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod thread_block;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod thread_exit;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod thread_list;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod thread_start;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod tp_veneer;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod thread_handlers;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod trampoline;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod unwind;
#[cfg(all(target_arch = "aarch64", target_os = "macos"))]
pub mod va_list;
#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod fixture_elf;
#[cfg(test)]
mod fixture_reference_tests;
#[cfg(test)]
mod fixture_header;
#[cfg(test)]
mod fixture_manifest_tests;
#[cfg(test)]
mod fixture_jni_tests;
#[cfg(test)]
mod fixture_layout_tests;
#[cfg(all(test, unix))]
mod fixture_script_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod fixture_exec_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod loader_test_support;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod loader_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod loader_corpus_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod loader_link_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod image_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod loader_protection_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod loader_summary_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod bridge_test_support;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod bridge_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod bridge_va_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod guest_thread_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod guest_call_tests;
#[cfg(test)]
mod test_deadline;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod fork_gate_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod signal_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod futex_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_test_support;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_mutex_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_cond_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_key_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_tls_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_lifecycle_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_semaphore_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_barrier_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_rwlock_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_once_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_futex_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_prctl_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod thread_corpus_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod tp_veneer_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod errno_map_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod data_fault_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod runtime_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod bridge_va_composite_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod libc_core_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod libc_repair_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod bionic_reference;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod libc_runtime_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod libc_system_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod libc_dl_math_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod libc_text_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod libc_coverage_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod libc_time_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod unwind_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod phdr_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod cxx_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod cxx_corpus_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod io_test_support;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod io_fs_tests;
#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod io_runtime_tests;
