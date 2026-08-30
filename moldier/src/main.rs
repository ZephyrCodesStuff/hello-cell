//! moldier - PlayStation 3 ELF Post-Linker and SPRX Tool for mold
//!
//! Applies Sony LV2 kernel & PRX runtime fixes to standard PowerPC64 ELFv1 binaries
//! produced by mold or other standard ELF linkers without requiring custom linker scripts.

use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELFCLASS64: u8 = 2;
const ELFDATA2MSB: u8 = 2; // Big Endian
const EM_PPC64: u16 = 21; // 0x15
const ELFOSABI_CELLLV2: u8 = 102; // 0x66

const PT_SCE_PROC_PARAM: u32 = 0x6000_0001;
const PT_SCE_PROC_PRX_PARAM: u32 = 0x6000_0002;

// -----------------------------------------------------------------------------
// Declarative SPRX Configuration Types
// -----------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
pub struct SprxConfig {
    #[serde(default)]
    pub process: ProcessConfig,
    #[serde(default)]
    pub libraries: BTreeMap<String, LibraryConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ProcessConfig {
    #[serde(default = "default_priority")]
    pub primary_prio: u32,
    #[serde(default = "default_stack_size")]
    pub primary_stack_size: u32,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            primary_prio: 1001,
            primary_stack_size: 0x0010_0000,
        }
    }
}

fn default_priority() -> u32 {
    1001
}
fn default_stack_size() -> u32 {
    0x0010_0000
} // 1 MB

#[derive(Debug, Deserialize)]
pub struct LibraryConfig {
    #[serde(default = "default_module_id")]
    pub module_id: u16,
    pub functions: Vec<FunctionConfig>,
}

fn default_module_id() -> u16 {
    0x0009
}

#[derive(Debug, Deserialize)]
pub struct FunctionConfig {
    pub name: String,
    pub fnid: u32,
}

pub fn generate_sprx_assembly(config: &SprxConfig) -> String {
    let mut out = String::new();
    out.push_str("# Auto-generated PlayStation 3 System & SPRX Stubs by moldier\n");
    out.push_str("# DO NOT EDIT DIRECTLY: Generated from sprx.toml\n\n");

    // 0. System Process & PRX Parameters (PT_SCE_PROC_PARAM / PT_SCE_PROC_PRX_PARAM)
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str(
        "# Process Parameter Block (Required by PS3 GameOS / Debug Agent to spawn process)\n",
    );
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str(".section \".sys_proc_param\",\"a\"\n.align 3\n.globl sys_process_param\nsys_process_param:\n");
    out.push_str("    .long 0x00000020        # size = 32 bytes\n");
    out.push_str("    .long 0x13bcc5f6        # magic = SYS_PROCESS_SPAWN_MAGIC\n");
    out.push_str("    .long 0x00009000        # version = SYS_PROCESS_SPAWN_VERSION_090\n");
    out.push_str("    .long 0x00192001        # sdk_version = SYS_PROCESS_SPAWN_FW_VERSION_192\n");
    out.push_str(&format!(
        "    .long {}              # priority\n",
        config.process.primary_prio
    ));
    out.push_str(&format!(
        "    .long 0x{:08X}        # stack size\n",
        config.process.primary_stack_size
    ));
    out.push_str("    .long 0x00100000        # malloc pagesize = 1 MB (0x100000)\n");
    out.push_str("    .long 0x00000000        # ppc_seg = SYS_PROCESS_SPAWN_PPC_SEG_PRX\n\n");

    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str(
        "# PRX Loader Parameters (Points to .lib.stub import table - resolved by moldier)\n",
    );
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str(".section \".sys_proc_prx_param\",\"a\"\n.align 2\n.globl sys_proc_prx_param\nsys_proc_prx_param:\n");
    out.push_str("    .long 0x00000028        # size = 40 bytes\n");
    out.push_str("    .long 0x1b434cec        # magic\n");
    out.push_str("    .long 0x00000002        # version\n");
    out.push_str("    .long 0x00000000        # sdk_version\n");
    out.push_str("    .long 0x00000000        # libent_start (patched by moldier)\n");
    out.push_str("    .long 0x00000000        # libent_end   (patched by moldier)\n");
    out.push_str("    .long 0x00000000        # libstub_start (patched by moldier)\n");
    out.push_str("    .long 0x00000000        # libstub_end   (patched by moldier)\n");
    out.push_str("    .short 0x0101           # flags\n");
    out.push_str("    .short 0x0000\n");
    out.push_str("    .long 0x00000000\n\n");

    // 1. Library Names (.rodata.sceResident)
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str("# Library Names\n");
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str(".section \".rodata.sceResident\",\"a\"\n");
    for lib_name in config.libraries.keys() {
        out.push_str(&format!(
            ".globl {}_name\n{}_name:\n    .asciz \"{}\"\n",
            lib_name, lib_name, lib_name
        ));
    }
    out.push('\n');

    // 2. FNID Hash Tables (.rodata.sceFNID)
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str("# FNID Hash Tables\n");
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str(".section \".rodata.sceFNID\",\"a\"\n.align 2\n\n");
    for (lib_name, lib) in &config.libraries {
        out.push_str(&format!(
            ".globl {}_fnid_table\n{}_fnid_table:\n",
            lib_name, lib_name
        ));
        for func in &lib.functions {
            out.push_str(&format!("    .int 0x{:08X}   # {}\n", func.fnid, func.name));
        }
        out.push_str(&format!(
            ".globl {}_fnid_table_end\n{}_fnid_table_end:\n\n",
            lib_name, lib_name
        ));
    }

    // 3. 32-bit Stub Slots (.data.sceFStub.<lib>)
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str("# Function Pointer Slots (In .data.sceFStub: 32-bit pointer slots populated by LV2 PRX loader)\n");
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    for (lib_name, lib) in &config.libraries {
        out.push_str(&format!(
            ".section \".data.sceFStub.{}\",\"aw\"\n.align 2\n",
            lib_name
        ));
        out.push_str(&format!(
            ".globl {}_fstub_table\n{}_fstub_table:\n",
            lib_name, lib_name
        ));
        for func in &lib.functions {
            out.push_str(&format!(
                ".globl {}_stub\n{}_stub:\n    .int 0\n",
                func.name, func.name
            ));
        }
        out.push_str(&format!(
            ".globl {}_fstub_table_end\n{}_fstub_table_end:\n\n",
            lib_name, lib_name
        ));
    }

    // 4. PRX Import Headers (.lib.stub)
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str("# PRX Import Headers (Read by PS3 LV2 Kernel Loader - patched by moldier)\n");
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str(".section \".lib.stub\",\"aw\"\n.align 2\n\n");
    for (lib_name, lib) in &config.libraries {
        out.push_str(&format!(
            ".globl {}_prx_header\n{}_prx_header:\n",
            lib_name, lib_name
        ));
        out.push_str("    .int 0x2c000001\n");
        out.push_str(&format!("    .short 0x{:04x}\n", lib.module_id));
        out.push_str(&format!("    .short {}\n", lib.functions.len()));
        out.push_str("    .int 0, 0\n");
        out.push_str("    .int 0            # name (patched by moldier)\n");
        out.push_str("    .int 0            # fnid (patched by moldier)\n");
        out.push_str("    .int 0            # fstub (patched by moldier)\n");
        out.push_str("    .int 0, 0, 0, 0\n\n");
    }

    // 5. Trampolines (.sceStub.text)
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str("# Trampoline Functions & ELFv1 OPD Descriptors\n");
    out.push_str(
        "# -----------------------------------------------------------------------------\n",
    );
    out.push_str(".section \".sceStub.text\",\"ax\"\n.align 2\n\n");
    out.push_str(
        ".macro DEFINE_STUB name\n\
.align 2\n\
.globl \\name\n\
.type \\name, @function\n\
\\name:\n\
    mflr    0\n\
    std     0, 16(1)\n\
    std     2, 40(1)\n\
    stdu    1, -128(1)\n\
    addis   12, 2, \\name\\()_stub@toc@ha\n\
    addi    12, 12, \\name\\()_stub@toc@l\n\
    lwz     12, 0(12)                 # Load 32-bit pointer to runtime descriptor\n\
    lwz     0, 0(12)                  # Load resolved function entry address\n\
    lwz     2, 4(12)                  # Load resolved SPRX TOC base\n\
    mtctr   0\n\
    bctrl                             # Call into SPRX!\n\
    addi    1, 1, 128\n\
    ld      2, 40(1)                  # Restore our application TOC (r2)\n\
    ld      0, 16(1)\n\
    mtlr    0\n\
    blr\n\
.endm\n\n",
    );

    for (lib_name, lib) in &config.libraries {
        out.push_str(&format!("# {} stubs\n", lib_name));
        for func in &lib.functions {
            out.push_str(&format!("DEFINE_STUB {}\n", func.name));
        }
        out.push('\n');
    }

    out
}

// -----------------------------------------------------------------------------
// ELF Patcher Core
// -----------------------------------------------------------------------------
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct SectionHeader {
    name_idx: u32,
    name: String,
    sh_type: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entsize: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
struct ElfPatcher {
    data: Vec<u8>,
    entry_point: u64,
    phoff: u64,
    shoff: u64,
    phnum: u16,
    shnum: u16,
    shstrndx: u16,
    sections: HashMap<String, SectionHeader>,
    symbols: HashMap<String, u64>,
}

impl ElfPatcher {
    pub fn new(data: Vec<u8>) -> Result<Self, String> {
        if data.len() < 64 {
            return Err("File too small to be a valid ELF64 header".into());
        }

        if &data[0..4] != ELF_MAGIC {
            return Err("Invalid ELF magic".into());
        }

        if data[4] != ELFCLASS64 {
            return Err("Not a 64-bit ELF file".into());
        }

        if data[5] != ELFDATA2MSB {
            return Err("Not a Big-Endian (MSB) ELF file".into());
        }

        let machine = u16::from_be_bytes(data[18..20].try_into().unwrap());
        if machine != EM_PPC64 {
            return Err(format!(
                "Unsupported machine 0x{:04X} (expected EM_PPC64 = 0x0015)",
                machine
            ));
        }

        let entry_point = u64::from_be_bytes(data[24..32].try_into().unwrap());
        let phoff = u64::from_be_bytes(data[32..40].try_into().unwrap());
        let shoff = u64::from_be_bytes(data[40..48].try_into().unwrap());
        let phnum = u16::from_be_bytes(data[56..58].try_into().unwrap());
        let shentsize = u16::from_be_bytes(data[58..60].try_into().unwrap()) as usize;
        let shnum = u16::from_be_bytes(data[60..62].try_into().unwrap());
        let shstrndx = u16::from_be_bytes(data[62..64].try_into().unwrap());

        if shentsize != 64 {
            return Err(format!(
                "Unexpected section header entry size: {}",
                shentsize
            ));
        }

        // Read all section headers
        let mut raw_shdrs = Vec::new();
        for i in 0..shnum as usize {
            let start = (shoff as usize) + i * 64;
            let end = start + 64;
            if end > data.len() {
                return Err("Section header table exceeds file boundary".into());
            }
            let s = &data[start..end];
            let name_idx = u32::from_be_bytes(s[0..4].try_into().unwrap());
            let sh_type = u32::from_be_bytes(s[4..8].try_into().unwrap());
            let flags = u64::from_be_bytes(s[8..16].try_into().unwrap());
            let addr = u64::from_be_bytes(s[16..24].try_into().unwrap());
            let offset = u64::from_be_bytes(s[24..32].try_into().unwrap());
            let size = u64::from_be_bytes(s[32..40].try_into().unwrap());
            let link = u32::from_be_bytes(s[40..44].try_into().unwrap());
            let info = u32::from_be_bytes(s[44..48].try_into().unwrap());
            let addralign = u64::from_be_bytes(s[48..56].try_into().unwrap());
            let entsize = u64::from_be_bytes(s[56..64].try_into().unwrap());

            raw_shdrs.push((
                name_idx, sh_type, flags, addr, offset, size, link, info, addralign, entsize,
            ));
        }

        // Read section names from shstrtab
        if (shstrndx as usize) >= raw_shdrs.len() {
            return Err("Invalid shstrndx section index".into());
        }
        let shstrtab_info = &raw_shdrs[shstrndx as usize];
        let shstr_start = shstrtab_info.4 as usize;
        let shstr_end = shstr_start + (shstrtab_info.5 as usize);
        if shstr_end > data.len() {
            return Err("shstrtab exceeds file bounds".into());
        }
        let shstrtab = &data[shstr_start..shstr_end];

        let mut sections = HashMap::new();
        for (name_idx, sh_type, flags, addr, offset, size, link, info, addralign, entsize) in
            raw_shdrs
        {
            let name_bytes = &shstrtab[name_idx as usize..];
            let name_len = name_bytes
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(name_bytes.len());
            let name = String::from_utf8_lossy(&name_bytes[..name_len]).into_owned();

            sections.insert(
                name.clone(),
                SectionHeader {
                    name_idx,
                    name,
                    sh_type,
                    flags,
                    addr,
                    offset,
                    size,
                    link,
                    info,
                    addralign,
                    entsize,
                },
            );
        }

        // Parse symbol table if present
        let mut symbols = HashMap::new();
        if let (Some(sym_sec), Some(str_sec)) = (sections.get(".symtab"), sections.get(".strtab")) {
            let sym_start = sym_sec.offset as usize;
            let sym_end = sym_start + (sym_sec.size as usize);
            let str_start = str_sec.offset as usize;
            let str_end = str_start + (str_sec.size as usize);

            if sym_end <= data.len() && str_end <= data.len() {
                let sym_data = &data[sym_start..sym_end];
                let str_data = &data[str_start..str_end];
                let ent_size = if sym_sec.entsize > 0 {
                    sym_sec.entsize as usize
                } else {
                    24
                };
                let num_syms = sym_data.len() / ent_size;

                for i in 0..num_syms {
                    let s_off = i * ent_size;
                    let st_name =
                        u32::from_be_bytes(sym_data[s_off..s_off + 4].try_into().unwrap()) as usize;
                    let st_value =
                        u64::from_be_bytes(sym_data[s_off + 8..s_off + 16].try_into().unwrap());

                    if st_name < str_data.len() {
                        let name_bytes = &str_data[st_name..];
                        let name_len = name_bytes
                            .iter()
                            .position(|&b| b == 0)
                            .unwrap_or(name_bytes.len());
                        let name = String::from_utf8_lossy(&name_bytes[..name_len]).into_owned();
                        if !name.is_empty() {
                            symbols.insert(name, st_value);
                        }
                    }
                }
            }
        }

        Ok(Self {
            data,
            entry_point,
            phoff,
            shoff,
            phnum,
            shnum,
            shstrndx,
            sections,
            symbols,
        })
    }

    /// Fix 1: Set ELF OS/ABI to ELFOSABI_CELLLV2 (102 = 0x66)
    pub fn patch_osabi(&mut self) {
        self.data[7] = ELFOSABI_CELLLV2;
        println!(
            "  -> Set ELF OS/ABI to ELFOSABI_CELLLV2 (0x{:02X})",
            ELFOSABI_CELLLV2
        );
    }

    /// Fix 2: Sanitize PHDRs, strip non-PS3 GNU headers, set Sony flags, and inject PT_SCE headers
    pub fn sanitize_and_inject_phdrs(&mut self) -> Result<usize, String> {
        let phentsize = 56usize;
        let phoff = self.phoff as usize;
        let phnum = self.phnum as usize;

        let mut valid_phdrs: Vec<[u8; 56]> = Vec::new();

        // 1. Process existing PHDRs, keeping ONLY PT_LOAD (type 1)
        for i in 0..phnum {
            let offset = phoff + i * phentsize;
            if offset + phentsize <= self.data.len() {
                let p_type = u32::from_be_bytes(self.data[offset..offset + 4].try_into().unwrap());
                if p_type == 1 {
                    let mut ph_bytes = [0u8; 56];
                    ph_bytes.copy_from_slice(&self.data[offset..offset + 56]);

                    let p_flags = u32::from_be_bytes(ph_bytes[4..8].try_into().unwrap());
                    // Sony PPU memory flags
                    let new_flags = if (p_flags & 1) != 0 {
                        0x0040_0005u32 // PF_PPU_EXEC | PF_R | PF_X
                    } else if (p_flags & 2) != 0 {
                        0x0060_0006u32 // PF_PPU_DATA | PF_R | PF_W
                    } else {
                        p_flags
                    };
                    ph_bytes[4..8].copy_from_slice(&new_flags.to_be_bytes());

                    let p_vaddr = u64::from_be_bytes(ph_bytes[16..24].try_into().unwrap());
                    let p_filesz = u64::from_be_bytes(ph_bytes[32..40].try_into().unwrap());
                    println!(
                        "  -> Kept PT_LOAD segment (vaddr 0x{:X}, size 0x{:X}, flags 0x{:08X})",
                        p_vaddr, p_filesz, new_flags
                    );
                    valid_phdrs.push(ph_bytes);
                } else {
                    println!("  -> Stripped non-PS3 header 0x{:08X}", p_type);
                }
            }
        }

        // 2. Inject PT_SCE_PROC_PARAM (0x60000001)
        if let Some(param_sec) = self.sections.get(".sys_proc_param") {
            let mut ph_bytes = [0u8; 56];
            ph_bytes[0..4].copy_from_slice(&PT_SCE_PROC_PARAM.to_be_bytes());
            ph_bytes[4..8].copy_from_slice(&0u32.to_be_bytes()); // p_flags = 0
            ph_bytes[8..16].copy_from_slice(&param_sec.offset.to_be_bytes());
            ph_bytes[16..24].copy_from_slice(&param_sec.addr.to_be_bytes());
            ph_bytes[24..32].copy_from_slice(&param_sec.addr.to_be_bytes()); // p_paddr = p_vaddr
            ph_bytes[32..40].copy_from_slice(&param_sec.size.to_be_bytes());
            ph_bytes[40..48].copy_from_slice(&param_sec.size.to_be_bytes());
            ph_bytes[48..56].copy_from_slice(&param_sec.addralign.max(8).to_be_bytes());

            println!(
                "  -> Injected PT_SCE_PROC_PARAM (0x{:08X}) at 0x{:X}",
                PT_SCE_PROC_PARAM, param_sec.addr
            );
            valid_phdrs.push(ph_bytes);
        }

        // 3. Inject PT_SCE_PROC_PRX_PARAM (0x60000002)
        if let Some(prx_sec) = self.sections.get(".sys_proc_prx_param") {
            let mut ph_bytes = [0u8; 56];
            ph_bytes[0..4].copy_from_slice(&PT_SCE_PROC_PRX_PARAM.to_be_bytes());
            ph_bytes[4..8].copy_from_slice(&0u32.to_be_bytes()); // p_flags = 0
            ph_bytes[8..16].copy_from_slice(&prx_sec.offset.to_be_bytes());
            ph_bytes[16..24].copy_from_slice(&prx_sec.addr.to_be_bytes());
            ph_bytes[24..32].copy_from_slice(&prx_sec.addr.to_be_bytes()); // p_paddr = p_vaddr
            ph_bytes[32..40].copy_from_slice(&prx_sec.size.to_be_bytes());
            ph_bytes[40..48].copy_from_slice(&prx_sec.size.to_be_bytes());
            ph_bytes[48..56].copy_from_slice(&prx_sec.addralign.max(4).to_be_bytes());

            println!(
                "  -> Injected PT_SCE_PROC_PRX_PARAM (0x{:08X}) at 0x{:X}",
                PT_SCE_PROC_PRX_PARAM, prx_sec.addr
            );
            valid_phdrs.push(ph_bytes);
        }

        // Overwrite PHDR table cleanly
        for (i, ph) in valid_phdrs.iter().enumerate() {
            let offset = phoff + i * phentsize;
            self.data[offset..offset + 56].copy_from_slice(ph);
        }

        // Zero out any remaining old PHDR slots
        for i in valid_phdrs.len()..phnum {
            let offset = phoff + i * phentsize;
            self.data[offset..offset + 56].fill(0);
        }

        // Update phnum in ELF header (bytes 56..58)
        self.phnum = valid_phdrs.len() as u16;
        self.data[56..58].copy_from_slice(&self.phnum.to_be_bytes());
        println!("  -> Updated ELF e_phnum to {}", self.phnum);

        Ok(valid_phdrs.len())
    }

    /// Fix 3: SPRX Library Binding
    pub fn patch_lib_stubs(&mut self) -> Result<usize, String> {
        let stub_sec = match self.sections.get(".lib.stub") {
            Some(s) => s.clone(),
            None => return Ok(0), // No dynamic SPRX stubs in this binary
        };

        let stub_count = (stub_sec.size / 44) as usize;
        if stub_count == 0 {
            return Ok(0);
        }

        println!(
            "[moldier] Found .lib.stub ({} entries, offset 0x{:X})",
            stub_count, stub_sec.offset
        );

        // Find all symbols ending with `_prx_header` to automatically discover all libraries
        let mut prx_headers: Vec<(String, u64)> = self
            .symbols
            .iter()
            .filter_map(|(name, &vaddr)| {
                if let Some(prefix) = name.strip_suffix("_prx_header") {
                    Some((prefix.to_string(), vaddr))
                } else {
                    None
                }
            })
            .collect();
        prx_headers.sort_by_key(|&(_, vaddr)| vaddr);

        for (idx, (lib_name, header_vaddr)) in prx_headers.iter().enumerate() {
            if header_vaddr < &stub_sec.addr || header_vaddr >= &(stub_sec.addr + stub_sec.size) {
                continue;
            }
            let file_offset = (stub_sec.offset + (header_vaddr - stub_sec.addr)) as usize;
            if file_offset + 44 > self.data.len() {
                continue;
            }

            let sym_name = format!("{}_name", lib_name);
            let sym_fnid = format!("{}_fnid_table", lib_name);
            let sym_fstub = format!("{}_fstub_table", lib_name);
            let sym_fstub_end = format!("{}_fstub_table_end", lib_name);

            let name_ptr = self.symbols.get(&sym_name).copied().unwrap_or(0) as u32;
            let fnid_ptr = self.symbols.get(&sym_fnid).copied().unwrap_or(0) as u32;
            let fstub_ptr = self.symbols.get(&sym_fstub).copied().unwrap_or(0) as u32;

            let mut num_imports = u16::from_be_bytes(
                self.data[file_offset + 6..file_offset + 8]
                    .try_into()
                    .unwrap(),
            );
            if num_imports == 0 {
                if let (Some(&start), Some(&end)) = (
                    self.symbols.get(&sym_fstub),
                    self.symbols.get(&sym_fstub_end),
                ) {
                    num_imports = ((end.saturating_sub(start)) / 4) as u16;
                }
            }

            if name_ptr != 0 {
                self.data[file_offset + 16..file_offset + 20]
                    .copy_from_slice(&name_ptr.to_be_bytes());
            }
            if fnid_ptr != 0 {
                self.data[file_offset + 20..file_offset + 24]
                    .copy_from_slice(&fnid_ptr.to_be_bytes());
            }
            if fstub_ptr != 0 {
                self.data[file_offset + 24..file_offset + 28]
                    .copy_from_slice(&fstub_ptr.to_be_bytes());
            }
            if num_imports != 0 {
                self.data[file_offset + 6..file_offset + 8]
                    .copy_from_slice(&num_imports.to_be_bytes());
            }

            println!("  -> Library stub #{}: dynamically bound '{}' (num_imports={}, name=0x{:08X}, fnid=0x{:08X}, fstub=0x{:08X})",
                idx, lib_name, num_imports, name_ptr, fnid_ptr, fstub_ptr);
        }

        Ok(stub_count)
    }

    /// Fix 4: Pack OPD function descriptors
    pub fn patch_opd_descriptors(&mut self) -> Result<usize, String> {
        let opd_sec = match self.sections.get(".opd") {
            Some(s) => s.clone(),
            None => return Ok(0),
        };

        let count = (opd_sec.size / 24) as usize;
        if count == 0 {
            return Ok(0);
        }

        println!(
            "[moldier] Found .opd section: {} function descriptors ({} bytes)",
            count, opd_sec.size
        );
        let start_code_addr = self.symbols.get("_start_code").copied().unwrap_or(0);

        for i in 0..count {
            let offset = (opd_sec.offset as usize) + i * 24;
            if offset + 24 > self.data.len() {
                return Err(".opd entry exceeds file size".into());
            }

            let func_addr = u64::from_be_bytes(self.data[offset..offset + 8].try_into().unwrap());
            let rtoc = u64::from_be_bytes(self.data[offset + 8..offset + 16].try_into().unwrap());
            let is_entry_descriptor = (opd_sec.addr + i as u64 * 24) == self.entry_point
                || (start_code_addr != 0 && func_addr == start_code_addr);

            if is_entry_descriptor {
                // The PS3 kernel crt0 boots using 32-bit word loads (lwz r0, 0(r8); lwz r2, 4(r8)) on e_entry
                let func_u32 = (func_addr & 0xFFFF_FFFF) as u32;
                let rtoc_u32 = (rtoc & 0xFFFF_FFFF) as u32;
                self.data[offset..offset + 4].copy_from_slice(&func_u32.to_be_bytes());
                self.data[offset + 4..offset + 8].copy_from_slice(&rtoc_u32.to_be_bytes());
                let packed = (func_addr << 32) | (rtoc & 0xFFFF_FFFF);
                self.data[offset + 16..offset + 24].copy_from_slice(&packed.to_be_bytes());
                println!("  -> OPD descriptor #{}: configured entry descriptor (func=0x{:08X}, rtoc=0x{:08X})", i, func_u32, rtoc_u32);
            } else {
                // PS3 standard for all 64-bit Rust/C function pointer calls:
                // offset +0  (func): 64-bit function entry address
                // offset +8  (rtoc): 64-bit TOC (r2) base address
                // offset +16 (data): (func << 32) | (rtoc & 0xFFFFFFFF)
                let packed = (func_addr << 32) | (rtoc & 0xFFFF_FFFF);
                let packed_bytes = packed.to_be_bytes();
                self.data[offset + 16..offset + 24].copy_from_slice(&packed_bytes);
            }
        }

        println!("  -> Packed {} OPD descriptors", count);
        Ok(count)
    }

    /// Fix 6: Update `.sys_proc_prx_param` boundary pointers
    pub fn patch_prx_params(&mut self) -> Result<(), String> {
        let prx_sec = match self.sections.get(".sys_proc_prx_param") {
            Some(s) => s.clone(),
            None => return Ok(()),
        };

        if prx_sec.size < 0x28 {
            return Ok(());
        }

        let off = prx_sec.offset as usize;
        let libent_start = self
            .sections
            .get(".lib.ent")
            .map(|s| s.addr as u32)
            .unwrap_or(0);
        let libent_end = self
            .sections
            .get(".lib.ent")
            .map(|s| (s.addr + s.size) as u32)
            .unwrap_or(0);
        let libstub_start = self
            .sections
            .get(".lib.stub")
            .map(|s| s.addr as u32)
            .unwrap_or(0);
        let libstub_end = self
            .sections
            .get(".lib.stub")
            .map(|s| (s.addr + s.size) as u32)
            .unwrap_or(0);

        // Offsets in sys_proc_prx_param:
        // +16: libent_start
        // +20: libent_end
        // +24: libstub_start
        // +28: libstub_end
        self.data[off + 16..off + 20].copy_from_slice(&libent_start.to_be_bytes());
        self.data[off + 20..off + 24].copy_from_slice(&libent_end.to_be_bytes());
        self.data[off + 24..off + 28].copy_from_slice(&libstub_start.to_be_bytes());
        self.data[off + 28..off + 32].copy_from_slice(&libstub_end.to_be_bytes());

        println!("  -> Updated sys_proc_prx_param bounds: libent 0x{:08X}-0x{:08X}, libstub 0x{:08X}-0x{:08X}",
            libent_start, libent_end, libstub_start, libstub_end);

        Ok(())
    }

    /// Validate PS3 ELF alignment and critical sections
    pub fn validate(&self) {
        println!("[moldier] Validation Report:");
        println!("  • OS/ABI:      0x{:02X}", self.data[7]);
        println!("  • PHDR Count:  {}", self.phnum);
        println!("  • Entry Point: 0x{:016X}", self.entry_point);

        if let Some(text) = self.sections.get(".text") {
            println!(
                "  • .text:               0x{:08X} (size 0x{:X})",
                text.addr, text.size
            );
        }
        if let Some(opd) = self.sections.get(".opd") {
            println!(
                "  • .opd:                0x{:08X} (size 0x{:X})",
                opd.addr, opd.size
            );
        }
        if let Some(param) = self.sections.get(".sys_proc_param") {
            println!(
                "  • .sys_proc_param:     0x{:08X} (size 0x{:X})",
                param.addr, param.size
            );
        }
        if let Some(prx) = self.sections.get(".sys_proc_prx_param") {
            println!(
                "  • .sys_proc_prx_param: 0x{:08X} (size 0x{:X})",
                prx.addr, prx.size
            );
        }
        if let Some(stub) = self.sections.get(".lib.stub") {
            println!(
                "  • .lib.stub:           0x{:08X} (size 0x{:X})",
                stub.addr, stub.size
            );
        }
    }

    pub fn write_to_file(&self, path: &Path) -> io::Result<()> {
        fs::write(path, &self.data)
    }
}

fn patch_elf_file(input: &Path, output: Option<&Path>) -> Result<(), String> {
    let target_out = output.unwrap_or(input);
    println!("[moldier] Loading ELF: {}", input.display());

    let bytes = fs::read(input).map_err(|e| format!("Failed to read input file: {}", e))?;
    let mut patcher = ElfPatcher::new(bytes)?;

    patcher.patch_osabi();
    patcher.sanitize_and_inject_phdrs()?;
    patcher.patch_lib_stubs()?;
    patcher.patch_opd_descriptors()?;
    patcher.patch_prx_params()?;
    patcher.validate();

    patcher
        .write_to_file(target_out)
        .map_err(|e| format!("Failed to save output file: {}", e))?;
    println!(
        "[moldier] Saved patched PS3 ELF to: {}",
        target_out.display()
    );
    Ok(())
}

fn handle_gen_stubs(config_path: &Path, output_path: &Path) -> Result<(), String> {
    println!(
        "[moldier] Generating SPRX assembly stubs from: {}",
        config_path.display()
    );
    let toml_str = fs::read_to_string(config_path).map_err(|e| {
        format!(
            "Failed to read config file {}: {}",
            config_path.display(),
            e
        )
    })?;
    let config: SprxConfig = toml::from_str(&toml_str)
        .map_err(|e| format!("Failed to parse TOML in {}: {}", config_path.display(), e))?;

    let total_funcs: usize = config.libraries.values().map(|l| l.functions.len()).sum();
    println!(
        "[moldier] Found {} libraries with {} total functions",
        config.libraries.len(),
        total_funcs
    );

    let asm_code = generate_sprx_assembly(&config);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }
    fs::write(output_path, asm_code).map_err(|e| {
        format!(
            "Failed to write assembly output {}: {}",
            output_path.display(),
            e
        )
    })?;

    println!(
        "[moldier] Generated assembly written to: {}",
        output_path.display()
    );
    Ok(())
}

// -----------------------------------------------------------------------------
// CLI Definition (Clap)
// -----------------------------------------------------------------------------
#[derive(Parser)]
#[command(
    name = "moldier",
    author,
    version,
    about = "PlayStation 3 ELF Post-Linker and SPRX Tool for mold"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate SPRX assembly stubs (.s) from a declarative sprx.toml configuration
    GenStubs {
        /// Path to input sprx.toml
        #[arg(short, long, default_value = "sprx.toml")]
        config: PathBuf,
        /// Path to output assembly file
        #[arg(short, long, default_value = "src/sprx.s")]
        output: PathBuf,
    },
    /// Apply PS3 OPD, PHDR, and SPRX fixes to an ELF linked by mold
    Patch {
        /// Path to input ELF file
        input: PathBuf,
        /// Optional output path (defaults to overwriting input in-place)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match &cli.command {
        Commands::GenStubs { config, output } => {
            if let Err(err) = handle_gen_stubs(config, output) {
                eprintln!("[moldier] ERROR: {}", err);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Commands::Patch { input, output } => {
            if let Err(err) = patch_elf_file(input, output.as_deref()) {
                eprintln!("[moldier] ERROR: {}", err);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
    }
}
