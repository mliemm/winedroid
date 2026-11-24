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
