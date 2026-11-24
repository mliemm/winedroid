use crate::error::{Result, WinedroidError};

use super::dynamic_table::{DynamicInfo, Table};
use super::image_map::MappedImage;
use super::reloc_stream::{decode_aps2, decode_rela, decode_relr, Reloc};
use super::symbol_table::{Sym, SymbolTable, STB_LOCAL, STB_WEAK, STT_GNU_IFUNC, STT_TLS};
use super::tls_modules::{dynamic_resolver_address, unresolved_weak_resolver_address, TlsDescArg};

const R_AARCH64_NONE: u32 = 0;
const R_AARCH64_ABS64: u32 = 257;
const R_AARCH64_COPY: u32 = 1024;
const R_AARCH64_GLOB_DAT: u32 = 1025;
const R_AARCH64_JUMP_SLOT: u32 = 1026;
const R_AARCH64_RELATIVE: u32 = 1027;
const R_AARCH64_TLS_DTPMOD64: u32 = 1028;
const R_AARCH64_TLS_DTPREL64: u32 = 1029;
const R_AARCH64_TLS_TPREL64: u32 = 1030;
const R_AARCH64_TLSDESC: u32 = 1031;
const R_AARCH64_IRELATIVE: u32 = 1032;
const STT_SECTION: u8 = 3;

#[derive(Debug, Clone, Copy)]
pub struct Resolved {
    pub address: usize,
    pub kind: u8,
    pub value: u64,
    pub tls_module: u32,
}

pub trait SymbolResolution {
    fn resolve(&self, name: &str, sym: &Sym) -> Result<Option<Resolved>>;
}

pub struct RelocationContext<'a> {
    pub name: &'a str,
    pub image: &'a MappedImage,
    pub symbols: &'a SymbolTable,
    pub own_tls_module: u32,
    pub resolution: &'a dyn SymbolResolution,
    pub descriptors: &'a mut Vec<Box<TlsDescArg>>,
}

fn table_bytes<'a>(image: &'a MappedImage, table: &Table) -> Result<&'a [u8]> {
    if table.vaddr == 0 || table.size == 0 {
        return Ok(&[]);
    }
    image.bytes(table.vaddr, table.size)
}

pub fn relocate_all(context: &mut RelocationContext, dynamic: &DynamicInfo) -> Result<()> {
    // cite: bionic linker_relocate.cpp#relocate
    let image = context.image;
    for reloc in decode_aps2_table(image, &dynamic.android_rela)? {
        apply(context, &reloc)?;
    }
    let bias = image.load_bias() as u64;
    for offset in decode_relr(table_bytes(image, &dynamic.relr)?) {
        let value = image.read_u64(offset)?;
        image.write_u64(offset, value.wrapping_add(bias))?;
    }
    for reloc in decode_rela(table_bytes(image, &dynamic.rela)?) {
        apply(context, &reloc)?;
    }
    for reloc in decode_rela(table_bytes(image, &dynamic.jmprel)?) {
        apply(context, &reloc)?;
    }
    Ok(())
}

fn decode_aps2_table(image: &MappedImage, table: &Table) -> Result<Vec<Reloc>> {
    let bytes = table_bytes(image, table)?;
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    decode_aps2(bytes)
}

fn is_tls(kind: u32) -> bool {
    matches!(kind, R_AARCH64_TLS_DTPMOD64 | R_AARCH64_TLS_DTPREL64 | R_AARCH64_TLS_TPREL64 | R_AARCH64_TLSDESC)
}

fn apply(context: &mut RelocationContext, reloc: &Reloc) -> Result<()> {
    // cite: bionic linker_relocate.cpp#process_relocation_impl, process_relocation_general
    let image = context.image;
    let kind = reloc.kind();
    if kind == R_AARCH64_NONE {
        return Ok(());
    }
    let addend = reloc.addend as u64;
    let symbol_index = u64::from(reloc.symbol());
    let mut name = String::new();
    let mut sym = None;
    if symbol_index != 0 {
        let entry = context.symbols.symbol(image, symbol_index)?;
        name = context.symbols.name(image, &entry)?;
        sym = Some(entry);
    }
    if is_tls(kind) {
        return apply_tls(context, reloc, &name, sym.as_ref());
    }
    let mut address = 0u64;
    if let Some(entry) = &sym {
        match context.resolution.resolve(&name, entry)? {
            Some(resolved) if resolved.kind == STT_GNU_IFUNC => {
                return Err(WinedroidError::new(format!(
                    "STT_GNU_IFUNC symbol \"{name}\" referenced from \"{}\"; next missing primitive: STT_GNU_IFUNC symbol resolution",
                    context.name
                )));
            }
            Some(resolved) => address = resolved.address as u64,
            None => {}
        }
    }
    match kind {
        R_AARCH64_ABS64 | R_AARCH64_GLOB_DAT | R_AARCH64_JUMP_SLOT => image.write_u64(reloc.offset, address.wrapping_add(addend)),
        R_AARCH64_RELATIVE => image.write_u64(reloc.offset, (image.load_bias() as u64).wrapping_add(addend)),
        R_AARCH64_IRELATIVE => Err(WinedroidError::new(format!(
            "unsupported relocation type {kind} in \"{}\"; next missing primitive: R_AARCH64_IRELATIVE (ifunc)",
            context.name
        ))),
        R_AARCH64_COPY => Err(WinedroidError::new(format!("{} COPY relocations are not supported", context.name))),
        other => Err(WinedroidError::new(format!("unknown reloc type {other} in \"{}\"", context.name))),
    }
}

fn apply_tls(context: &mut RelocationContext, reloc: &Reloc, name: &str, sym: Option<&Sym>) -> Result<()> {
    // cite: bionic linker_relocate.cpp#process_relocation_impl, is_tls_reloc
    let image = context.image;
    let kind = reloc.kind();
    let addend = reloc.addend as u64;
    let mut tls_module = None;
    let mut sym_addr = 0u64;
    match sym {
        None => {
            if context.own_tls_module == 0 {
                return Err(WinedroidError::new(format!(
                    "TLS relocation refers to symbol \"\" in solib \"{}\" with no TLS segment",
                    context.name
                )));
            }
            tls_module = Some(context.own_tls_module);
        }
        Some(entry) if entry.bind() == STB_LOCAL => {
            let sym_type = entry.kind();
            return Err(WinedroidError::new(if sym_type == STT_SECTION {
                format!("unexpected TLS reference to local section in \"{}\": sym type {sym_type}, rel type {kind}", context.name)
            } else {
                format!(
                    "unexpected TLS reference to local symbol \"{name}\" in \"{}\": sym type {sym_type}, rel type {kind}",
                    context.name
                )
            }));
        }
        Some(entry) => {
            if let Some(resolved) = context.resolution.resolve(name, entry)? {
                if resolved.tls_module == 0 {
                    return Err(WinedroidError::new(format!(
                        "TLS relocation refers to symbol \"{name}\" in solib \"{}\" with no TLS segment",
                        context.name
                    )));
                }
                if resolved.kind != STT_TLS {
                    return Err(WinedroidError::new(format!(
                        "reference to non-TLS symbol \"{name}\" from TLS relocation in \"{}\"",
                        context.name
                    )));
                }
                tls_module = Some(resolved.tls_module);
                sym_addr = resolved.value;
            } else if entry.bind() != STB_WEAK {
                return Err(WinedroidError::new(format!("cannot locate TLS symbol \"{name}\" referenced by \"{}\"...", context.name)));
            }
        }
    }
    match kind {
        R_AARCH64_TLS_DTPMOD64 => image.write_u64(reloc.offset, u64::from(tls_module.unwrap_or(0))),
        R_AARCH64_TLS_DTPREL64 => image.write_u64(reloc.offset, sym_addr.wrapping_add(addend)),
        R_AARCH64_TLS_TPREL64 => match tls_module {
            None => image.write_u64(reloc.offset, sym_addr.wrapping_add(addend)),
            Some(_) => Err(WinedroidError::new(format!(
                "TLS symbol \"{name}\" in dlopened library referenced from \"{}\" using IE access model",
                context.name
            ))),
        },
        _ => {
            let (function, argument) = match tls_module {
                None => (unresolved_weak_resolver_address() as u64, addend),
                Some(module_id) => {
                    let mut arg = Box::new(TlsDescArg { generation: 0, module_id: u64::from(module_id), offset: sym_addr.wrapping_add(addend) });
                    let pointer = &mut *arg as *mut TlsDescArg as u64;
                    context.descriptors.push(arg);
                    (dynamic_resolver_address() as u64, pointer)
                }
            };
            image.write_u64(reloc.offset, function)?;
            image.write_u64(reloc.offset + 8, argument)
        }
    }
}
