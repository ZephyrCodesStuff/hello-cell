use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
struct SprxConfig {
    #[serde(default)]
    process: ProcessConfig,
    #[serde(default)]
    libraries: BTreeMap<String, LibraryConfig>,
}

#[derive(Debug, Deserialize)]
struct ProcessConfig {
    #[serde(default = "default_priority")]
    primary_prio: u32,
    #[serde(default = "default_stack_size")]
    primary_stack_size: u32,
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
}

#[derive(Debug, Deserialize)]
struct LibraryConfig {
    #[serde(default = "default_module_id")]
    module_id: u16,
    #[serde(default)]
    functions: Vec<FunctionConfig>,
}

fn default_module_id() -> u16 {
    0x0009
}

#[derive(Debug, Deserialize)]
struct FunctionConfig {
    name: String,
    fnid: u32,
}

fn generate_sprx_assembly(config: &SprxConfig) -> String {
    let mut out = String::new();
    out.push_str("# Auto-generated PlayStation 3 System & SPRX Stubs by ps3 build.rs\n\n");

    // 0. System Process & PRX Parameters
    out.push_str(".section \".sys_proc_param\",\"aR\"\n.align 3\n.globl sys_process_param\nsys_process_param:\n");
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

    out.push_str(".section \".sys_proc_prx_param\",\"aR\"\n.align 2\n.globl sys_proc_prx_param\nsys_proc_prx_param:\n");
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

    if config.libraries.is_empty() {
        return out;
    }

    // 1. Library Names (.rodata.sceResident)
    out.push_str(".section \".rodata.sceResident\",\"aR\"\n");
    for lib_name in config.libraries.keys() {
        out.push_str(&format!(
            ".globl {}_name\n{}_name:\n    .asciz \"{}\"\n",
            lib_name, lib_name, lib_name
        ));
    }
    out.push('\n');

    // 2. FNID Hash Tables (.rodata.sceFNID)
    out.push_str(".section \".rodata.sceFNID\",\"aR\"\n.align 2\n\n");
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
    out.push_str(".section \".lib.stub\",\"awR\"\n.align 2\n\n");
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
    lwz     12, 0(12)\n\
    lwz     0, 0(12)\n\
    lwz     2, 4(12)\n\
    mtctr   0\n\
    bctrl\n\
    addi    1, 1, 128\n\
    ld      2, 40(1)\n\
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

fn main() {
    println!("cargo:rerun-if-changed=sprx.toml");
    println!("cargo:rerun-if-changed=../sprx.toml");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = PathBuf::from(out_dir).join("sprx.s");

    let config_path = if fs::metadata("sprx.toml").is_ok() {
        Some(PathBuf::from("sprx.toml"))
    } else if fs::metadata("../sprx.toml").is_ok() {
        Some(PathBuf::from("../sprx.toml"))
    } else {
        None
    };

    let config: SprxConfig = if let Some(path) = config_path {
        let content = fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    } else {
        SprxConfig::default()
    };

    let asm = generate_sprx_assembly(&config);
    fs::write(dest_path, asm).expect("Failed to write generated sprx.s");
}
