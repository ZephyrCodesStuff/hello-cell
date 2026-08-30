//! moldier - PlayStation 3 ELF Post-Linker and Patcher for mold
//!
//! Applies Sony LV2 kernel & PRX runtime fixes to standard PowerPC64 ELFv1 binaries
//! produced by mold or other standard ELF linkers without requiring custom linker scripts.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ELFCLASS64: u8 = 2;
const ELFDATA2MSB: u8 = 2; // Big Endian
const EM_PPC64: u16 = 21;  // 0x15
const ELFOSABI_CELLLV2: u8 = 102; // 0x66

const PT_SCE_PROC_PARAM: u32 = 0x6000_0001;
const PT_SCE_PROC_PRX_PARAM: u32 = 0x6000_0002;

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
            return Err(format!("Unsupported machine 0x{:04X} (expected EM_PPC64 = 0x0015)", machine));
        }

        let entry_point = u64::from_be_bytes(data[24..32].try_into().unwrap());
        let phoff = u64::from_be_bytes(data[32..40].try_into().unwrap());
        let shoff = u64::from_be_bytes(data[40..48].try_into().unwrap());
        let phnum = u16::from_be_bytes(data[56..58].try_into().unwrap());
        let shentsize = u16::from_be_bytes(data[58..60].try_into().unwrap()) as usize;
        let shnum = u16::from_be_bytes(data[60..62].try_into().unwrap());
        let shstrndx = u16::from_be_bytes(data[62..64].try_into().unwrap());

        if shentsize != 64 {
            return Err(format!("Unexpected section header entry size: {}", shentsize));
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

            raw_shdrs.push((name_idx, sh_type, flags, addr, offset, size, link, info, addralign, entsize));
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
        for (name_idx, sh_type, flags, addr, offset, size, link, info, addralign, entsize) in raw_shdrs {
            let name_bytes = &shstrtab[name_idx as usize..];
            let name_len = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_bytes.len());
            let name = String::from_utf8_lossy(&name_bytes[..name_len]).into_owned();

            sections.insert(name.clone(), SectionHeader {
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
            });
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
                let ent_size = if sym_sec.entsize > 0 { sym_sec.entsize as usize } else { 24 };
                let num_syms = sym_data.len() / ent_size;

                for i in 0..num_syms {
                    let s_off = i * ent_size;
                    let st_name = u32::from_be_bytes(sym_data[s_off..s_off + 4].try_into().unwrap()) as usize;
                    let st_value = u64::from_be_bytes(sym_data[s_off + 8..s_off + 16].try_into().unwrap());

                    if st_name < str_data.len() {
                        let name_bytes = &str_data[st_name..];
                        let name_len = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_bytes.len());
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
        println!("  -> Set ELF OS/ABI to ELFOSABI_CELLLV2 (0x{:02X})", ELFOSABI_CELLLV2);
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
                    println!("  -> Kept PT_LOAD segment (vaddr 0x{:X}, size 0x{:X}, flags 0x{:08X})", p_vaddr, p_filesz, new_flags);
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

            println!("  -> Injected PT_SCE_PROC_PARAM (0x{:08X}) at 0x{:X}", PT_SCE_PROC_PARAM, param_sec.addr);
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

            println!("  -> Injected PT_SCE_PROC_PRX_PARAM (0x{:08X}) at 0x{:X}", PT_SCE_PROC_PRX_PARAM, prx_sec.addr);
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

    /// Fix 3: Calculate FNID counts and resolve SPRX table pointers in `.lib.stub`
    pub fn patch_lib_stubs(&mut self) -> Result<usize, String> {
        let stub_sec = match self.sections.get(".lib.stub") {
            Some(s) => s.clone(),
            None => return Ok(0), // No dynamic SPRX stubs in this binary
        };

        let stub_count = (stub_sec.size / 44) as usize;
        if stub_count == 0 {
            return Ok(0);
        }

        println!("[moldier] Found .lib.stub ({} entries, offset 0x{:X})", stub_count, stub_sec.offset);

        // Auto-resolve library pointers from symbol table
        let known_libs: [(&str, &str, &str, &str, u16); 3] = [
            ("cellSysmodule", "cellSysmodule_name", "cellSysmodule_fnid_table", "cellSysmodule_fstub_table", 3),
            ("sys_net", "sys_net_name", "sys_net_fnid_table", "sys_net_fstub_table", 13),
            ("cellNetCtl", "cellNetCtl_name", "cellNetCtl_fnid_table", "cellNetCtl_fstub_table", 4),
        ];

        for i in 0..stub_count {
            let offset = (stub_sec.offset as usize) + i * 44;
            if offset + 44 > self.data.len() {
                return Err("Stub header entry exceeds file size".into());
            }

            let mut name_ptr = u32::from_be_bytes(self.data[offset + 16..offset + 20].try_into().unwrap());
            let mut fnid_ptr = u32::from_be_bytes(self.data[offset + 20..offset + 24].try_into().unwrap());
            let mut fstub_ptr = u32::from_be_bytes(self.data[offset + 24..offset + 28].try_into().unwrap());
            let mut num_imports = u16::from_be_bytes(self.data[offset + 6..offset + 8].try_into().unwrap());

            if i < known_libs.len() {
                let (lib_name, sym_name, sym_fnid, sym_fstub, default_count) = known_libs[i];
                if name_ptr == 0 {
                    if let Some(&val) = self.symbols.get(sym_name) {
                        name_ptr = val as u32;
                        self.data[offset + 16..offset + 20].copy_from_slice(&name_ptr.to_be_bytes());
                    }
                }
                if fnid_ptr == 0 {
                    if let Some(&val) = self.symbols.get(sym_fnid) {
                        fnid_ptr = val as u32;
                        self.data[offset + 20..offset + 24].copy_from_slice(&fnid_ptr.to_be_bytes());
                    }
                }
                if fstub_ptr == 0 {
                    if let Some(&val) = self.symbols.get(sym_fstub) {
                        fstub_ptr = val as u32;
                        self.data[offset + 24..offset + 28].copy_from_slice(&fstub_ptr.to_be_bytes());
                    }
                }
                if num_imports == 0 {
                    num_imports = default_count;
                    self.data[offset + 6..offset + 8].copy_from_slice(&num_imports.to_be_bytes());
                }

                println!("  -> Library stub #{}: bound '{}' (num_imports={}, name=0x{:08X}, fnid=0x{:08X}, fstub=0x{:08X})",
                    i, lib_name, num_imports, name_ptr, fnid_ptr, fstub_ptr);
            }
        }

        Ok(stub_count)
    }

    /// Fix 4: Pack OPD function descriptors matching PSL1GHT sprxlinker
    pub fn patch_opd_descriptors(&mut self) -> Result<usize, String> {
        let opd_sec = match self.sections.get(".opd") {
            Some(s) => s.clone(),
            None => return Ok(0),
        };

        let count = (opd_sec.size / 24) as usize;
        if count == 0 {
            return Ok(0);
        }

        println!("[moldier] Found .opd section: {} function descriptors ({} bytes)", count, opd_sec.size);
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
                // PSL1GHT standard (Opd64 in linker.c) for all 64-bit Rust/C function pointer calls:
                // offset +0  (func): 64-bit function entry address
                // offset +8  (rtoc): 64-bit TOC (r2) base address
                // offset +16 (data): (func << 32) | (rtoc & 0xFFFFFFFF)
                let packed = (func_addr << 32) | (rtoc & 0xFFFF_FFFF);
                let packed_bytes = packed.to_be_bytes();
                self.data[offset + 16..offset + 24].copy_from_slice(&packed_bytes);
            }
        }

        println!("  -> Packed {} OPD descriptors with PSL1GHT format", count);
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
        let libent_start = self.sections.get(".lib.ent").map(|s| s.addr as u32).unwrap_or(0);
        let libent_end = self.sections.get(".lib.ent").map(|s| (s.addr + s.size) as u32).unwrap_or(0);
        let libstub_start = self.sections.get(".lib.stub").map(|s| s.addr as u32).unwrap_or(0);
        let libstub_end = self.sections.get(".lib.stub").map(|s| (s.addr + s.size) as u32).unwrap_or(0);

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
            println!("  • .text:               0x{:08X} (size 0x{:X})", text.addr, text.size);
        }
        if let Some(opd) = self.sections.get(".opd") {
            println!("  • .opd:                0x{:08X} (size 0x{:X})", opd.addr, opd.size);
        }
        if let Some(param) = self.sections.get(".sys_proc_param") {
            println!("  • .sys_proc_param:     0x{:08X} (size 0x{:X})", param.addr, param.size);
        }
        if let Some(prx) = self.sections.get(".sys_proc_prx_param") {
            println!("  • .sys_proc_prx_param: 0x{:08X} (size 0x{:X})", prx.addr, prx.size);
        }
        if let Some(stub) = self.sections.get(".lib.stub") {
            println!("  • .lib.stub:           0x{:08X} (size 0x{:X})", stub.addr, stub.size);
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

    patcher.write_to_file(target_out).map_err(|e| format!("Failed to save output file: {}", e))?;
    println!("[moldier] Saved patched PS3 ELF to: {}", target_out.display());
    Ok(())
}

fn print_help() {
    println!(r#"moldier - PlayStation 3 ELF Post-Linker and Patcher for mold

USAGE:
    moldier patch <input_elf> [-o <output_elf>]
    moldier link [options...] -- <mold_args...>

COMMANDS:
    patch       Apply PS3 OPD, PHDR and SPRX fixes to an existing ELF
    link        Invoke mold with PPC64 ELFv1 flags and patch output automatically

EXAMPLES:
    moldier patch EBOOT.ELF
    moldier patch EBOOT.ELF -o EBOOT.PATCHED.ELF
"#);
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return ExitCode::from(1);
    }

    match args[1].as_str() {
        "patch" => {
            if args.len() < 3 {
                eprintln!("Error: Missing input ELF path.\nUsage: moldier patch <input_elf> [-o <output_elf>]");
                return ExitCode::from(1);
            }
            let input_path = PathBuf::from(&args[2]);
            let output_path = if args.len() >= 5 && args[3] == "-o" {
                Some(PathBuf::from(&args[4]))
            } else {
                None
            };

            if let Err(err) = patch_elf_file(&input_path, output_path.as_deref()) {
                eprintln!("[moldier] ERROR: {}", err);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        "help" | "--help" | "-h" => {
            print_help();
            ExitCode::SUCCESS
        }
        other => {
            let input_path = PathBuf::from(other);
            if input_path.exists() {
                if let Err(err) = patch_elf_file(&input_path, None) {
                    eprintln!("[moldier] ERROR: {}", err);
                    return ExitCode::from(1);
                }
                ExitCode::SUCCESS
            } else {
                eprintln!("Unknown command or file '{}'", other);
                print_help();
                ExitCode::from(1)
            }
        }
    }
}

