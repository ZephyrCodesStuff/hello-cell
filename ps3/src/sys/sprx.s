# Auto-generated PlayStation 3 System & SPRX Stubs by moldier
# DO NOT EDIT DIRECTLY: Generated from sprx.toml

# -----------------------------------------------------------------------------
# Process Parameter Block (Required by PS3 GameOS / Debug Agent to spawn process)
# -----------------------------------------------------------------------------
.section ".sys_proc_param","a"
.align 3
.globl sys_process_param
sys_process_param:
    .long 0x00000020        # size = 32 bytes
    .long 0x13bcc5f6        # magic = SYS_PROCESS_SPAWN_MAGIC
    .long 0x00009000        # version = SYS_PROCESS_SPAWN_VERSION_090
    .long 0x00192001        # sdk_version = SYS_PROCESS_SPAWN_FW_VERSION_192
    .long 1001              # priority
    .long 0x00100000        # stack size
    .long 0x00100000        # malloc pagesize = 1 MB (0x100000)
    .long 0x00000000        # ppc_seg = SYS_PROCESS_SPAWN_PPC_SEG_PRX

# -----------------------------------------------------------------------------
# PRX Loader Parameters (Points to .lib.stub import table - resolved by moldier)
# -----------------------------------------------------------------------------
.section ".sys_proc_prx_param","a"
.align 2
.globl sys_proc_prx_param
sys_proc_prx_param:
    .long 0x00000028        # size = 40 bytes
    .long 0x1b434cec        # magic
    .long 0x00000002        # version
    .long 0x00000000        # sdk_version
    .long 0x00000000        # libent_start (patched by moldier)
    .long 0x00000000        # libent_end   (patched by moldier)
    .long 0x00000000        # libstub_start (patched by moldier)
    .long 0x00000000        # libstub_end   (patched by moldier)
    .short 0x0101           # flags
    .short 0x0000
    .long 0x00000000

# -----------------------------------------------------------------------------
# Library Names
# -----------------------------------------------------------------------------
.section ".rodata.sceResident","a"
.globl cellNetCtl_name
cellNetCtl_name:
    .asciz "cellNetCtl"
.globl cellSysmodule_name
cellSysmodule_name:
    .asciz "cellSysmodule"
.globl sys_net_name
sys_net_name:
    .asciz "sys_net"

# -----------------------------------------------------------------------------
# FNID Hash Tables
# -----------------------------------------------------------------------------
.section ".rodata.sceFNID","a"
.align 2

.globl cellNetCtl_fnid_table
cellNetCtl_fnid_table:
    .int 0xBD5A59FC   # cellNetCtlInit
    .int 0x105EE2CB   # cellNetCtlTerm
    .int 0x8B3EBA69   # cellNetCtlGetState
    .int 0x1E585B5D   # cellNetCtlGetInfo
.globl cellNetCtl_fnid_table_end
cellNetCtl_fnid_table_end:

.globl cellSysmodule_fnid_table
cellSysmodule_fnid_table:
    .int 0x32267A31   # sysModuleLoad
    .int 0x112A5EE9   # sysModuleUnload
    .int 0x5A59E258   # sysModuleIsLoaded
.globl cellSysmodule_fnid_table_end
cellSysmodule_fnid_table_end:

.globl sys_net_fnid_table
sys_net_fnid_table:
    .int 0x139A9E9B   # netInitializeNetworkEx
    .int 0xB68D5625   # netFinalizeNetwork
    .int 0x9C056962   # netSocket
    .int 0x64F66D35   # netConnect
    .int 0xB0A59804   # netBind
    .int 0x28E208BB   # netListen
    .int 0xC94F6939   # netAccept
    .int 0xDC751B40   # netSend
    .int 0x9647570B   # netSendTo
    .int 0xFBA04F37   # netRecv
    .int 0x1F953B9F   # netRecvFrom
    .int 0xA50777C6   # netShutdown
    .int 0x6DB6E8CD   # netClose
.globl sys_net_fnid_table_end
sys_net_fnid_table_end:

# -----------------------------------------------------------------------------
# Function Pointer Slots (In .data.sceFStub: 32-bit pointer slots populated by LV2 PRX loader)
# -----------------------------------------------------------------------------
.section ".data.sceFStub.cellNetCtl","aw"
.align 2
.globl cellNetCtl_fstub_table
cellNetCtl_fstub_table:
.globl cellNetCtlInit_stub
cellNetCtlInit_stub:
    .int 0
.globl cellNetCtlTerm_stub
cellNetCtlTerm_stub:
    .int 0
.globl cellNetCtlGetState_stub
cellNetCtlGetState_stub:
    .int 0
.globl cellNetCtlGetInfo_stub
cellNetCtlGetInfo_stub:
    .int 0
.globl cellNetCtl_fstub_table_end
cellNetCtl_fstub_table_end:

.section ".data.sceFStub.cellSysmodule","aw"
.align 2
.globl cellSysmodule_fstub_table
cellSysmodule_fstub_table:
.globl sysModuleLoad_stub
sysModuleLoad_stub:
    .int 0
.globl sysModuleUnload_stub
sysModuleUnload_stub:
    .int 0
.globl sysModuleIsLoaded_stub
sysModuleIsLoaded_stub:
    .int 0
.globl cellSysmodule_fstub_table_end
cellSysmodule_fstub_table_end:

.section ".data.sceFStub.sys_net","aw"
.align 2
.globl sys_net_fstub_table
sys_net_fstub_table:
.globl netInitializeNetworkEx_stub
netInitializeNetworkEx_stub:
    .int 0
.globl netFinalizeNetwork_stub
netFinalizeNetwork_stub:
    .int 0
.globl netSocket_stub
netSocket_stub:
    .int 0
.globl netConnect_stub
netConnect_stub:
    .int 0
.globl netBind_stub
netBind_stub:
    .int 0
.globl netListen_stub
netListen_stub:
    .int 0
.globl netAccept_stub
netAccept_stub:
    .int 0
.globl netSend_stub
netSend_stub:
    .int 0
.globl netSendTo_stub
netSendTo_stub:
    .int 0
.globl netRecv_stub
netRecv_stub:
    .int 0
.globl netRecvFrom_stub
netRecvFrom_stub:
    .int 0
.globl netShutdown_stub
netShutdown_stub:
    .int 0
.globl netClose_stub
netClose_stub:
    .int 0
.globl sys_net_fstub_table_end
sys_net_fstub_table_end:

# -----------------------------------------------------------------------------
# PRX Import Headers (Read by PS3 LV2 Kernel Loader - patched by moldier)
# -----------------------------------------------------------------------------
.section ".lib.stub","aw"
.align 2

.globl cellNetCtl_prx_header
cellNetCtl_prx_header:
    .int 0x2c000001
    .short 0x0009
    .short 4
    .int 0, 0
    .int 0            # name (patched by moldier)
    .int 0            # fnid (patched by moldier)
    .int 0            # fstub (patched by moldier)
    .int 0, 0, 0, 0

.globl cellSysmodule_prx_header
cellSysmodule_prx_header:
    .int 0x2c000001
    .short 0x0009
    .short 3
    .int 0, 0
    .int 0            # name (patched by moldier)
    .int 0            # fnid (patched by moldier)
    .int 0            # fstub (patched by moldier)
    .int 0, 0, 0, 0

.globl sys_net_prx_header
sys_net_prx_header:
    .int 0x2c000001
    .short 0x0009
    .short 13
    .int 0, 0
    .int 0            # name (patched by moldier)
    .int 0            # fnid (patched by moldier)
    .int 0            # fstub (patched by moldier)
    .int 0, 0, 0, 0

# -----------------------------------------------------------------------------
# Trampoline Functions & ELFv1 OPD Descriptors
# -----------------------------------------------------------------------------
.section ".sceStub.text","ax"
.align 2

.macro DEFINE_STUB name
.align 2
.globl \name
.type \name, @function
\name:
mflr    0
std     0, 16(1)
std     2, 40(1)
stdu    1, -128(1)
addis   12, 2, \name\()_stub@toc@ha
addi    12, 12, \name\()_stub@toc@l
lwz     12, 0(12)                 # Load 32-bit pointer to runtime descriptor
lwz     0, 0(12)                  # Load resolved function entry address
lwz     2, 4(12)                  # Load resolved SPRX TOC base
mtctr   0
bctrl                             # Call into SPRX!
addi    1, 1, 128
ld      2, 40(1)                  # Restore our application TOC (r2)
ld      0, 16(1)
mtlr    0
blr
.endm

# cellNetCtl stubs
DEFINE_STUB cellNetCtlInit
DEFINE_STUB cellNetCtlTerm
DEFINE_STUB cellNetCtlGetState
DEFINE_STUB cellNetCtlGetInfo

# cellSysmodule stubs
DEFINE_STUB sysModuleLoad
DEFINE_STUB sysModuleUnload
DEFINE_STUB sysModuleIsLoaded

# sys_net stubs
DEFINE_STUB netInitializeNetworkEx
DEFINE_STUB netFinalizeNetwork
DEFINE_STUB netSocket
DEFINE_STUB netConnect
DEFINE_STUB netBind
DEFINE_STUB netListen
DEFINE_STUB netAccept
DEFINE_STUB netSend
DEFINE_STUB netSendTo
DEFINE_STUB netRecv
DEFINE_STUB netRecvFrom
DEFINE_STUB netShutdown
DEFINE_STUB netClose

