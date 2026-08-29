# PlayStation 3 Process Parameters & SPRX Import Tables
# Implements open-source PSL1GHT process parameters and FNID dynamic linking.

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
    .long 0x00100000        # stack size = 1 MB (0x100000)
    .long 0x00100000        # malloc pagesize = 1 MB (0x100000)
    .long 0x00000000        # ppc_seg = SYS_PROCESS_SPAWN_PPC_SEG_PRX

# -----------------------------------------------------------------------------
# PRX Loader Parameters (Points to .lib.stub import table)
# -----------------------------------------------------------------------------
.section ".sys_proc_prx_param","a"
.align 2
.globl sys_proc_prx_param
sys_proc_prx_param:
    .long 0x28
    .long 0x1b434cec
    .long 2
    .long 0
    .long __libentstart
    .long __libentend
    .long __libstubstart
    .long __libstubend
    .short 0x0101
    .short 0
    .long 0

# -----------------------------------------------------------------------------
# Library Names
# -----------------------------------------------------------------------------
.section ".rodata.sceResident","a"
cellSysmodule_name:
    .asciz "cellSysmodule"
sys_net_name:
    .asciz "sys_net"

# -----------------------------------------------------------------------------
# FNID Hash Tables
# -----------------------------------------------------------------------------
.section ".rodata.sceFNID","a"
.align 2

cellSysmodule_fnid_table:
    .int 0x32267A31   # sysModuleLoad
    .int 0x112A5EE9   # sysModuleUnload
    .int 0x5a59e258   # sysModuleIsLoaded

sys_net_fnid_table:
    .int 0x139a9e9b   # netInitializeNetworkEx
    .int 0xb68d5625   # netFinalizeNetwork
    .int 0x9c056962   # netSocket
    .int 0x64f66d35   # netConnect
    .int 0xb0a59804   # netBind
    .int 0x28e208bb   # netListen
    .int 0xc94f6939   # netAccept
    .int 0xdc751b40   # netSend
    .int 0x9647570b   # netSendTo
    .int 0xfba04f37   # netRecv
    .int 0x1f953b9f   # netRecvFrom
    .int 0xa50777c6   # netShutdown
    .int 0x6db6e8cd   # netClose

# -----------------------------------------------------------------------------
# Function Pointer Slots (In .data.sceFStub: 4 bytes per function, initialized to trampoline)
# At load time, the PS3 LV2 loader writes the address of the resolved runtime descriptor here.
# -----------------------------------------------------------------------------
.section ".data.sceFStub.cellSysmodule","aw"
.align 2
cellSysmodule_fstub_table:
sysModuleLoad_stub:
    .long __sysModuleLoad
sysModuleUnload_stub:
    .long __sysModuleUnload
sysModuleIsLoaded_stub:
    .long __sysModuleIsLoaded

.section ".data.sceFStub.sys_net","aw"
.align 2
sys_net_fstub_table:
netInitializeNetworkEx_stub:
    .long __netInitializeNetworkEx
netFinalizeNetwork_stub:
    .long __netFinalizeNetwork
netSocket_stub:
    .long __netSocket
netConnect_stub:
    .long __netConnect
netBind_stub:
    .long __netBind
netListen_stub:
    .long __netListen
netAccept_stub:
    .long __netAccept
netSend_stub:
    .long __netSend
netSendTo_stub:
    .long __netSendTo
netRecv_stub:
    .long __netRecv
netRecvFrom_stub:
    .long __netRecvFrom
netShutdown_stub:
    .long __netShutdown
netClose_stub:
    .long __netClose

# -----------------------------------------------------------------------------
# PRX Import Headers (Read by PS3 LV2 Kernel Loader)
# -----------------------------------------------------------------------------
.section ".lib.stub","aw"
.align 2

cellSysmodule_prx_header:
    .int 0x2c000001
    .short 0x0009
    .short 3          # 3 functions
    .int 0, 0
    .int cellSysmodule_name
    .int cellSysmodule_fnid_table
    .int cellSysmodule_fstub_table
    .int 0, 0, 0, 0

sys_net_prx_header:
    .int 0x2c000001
    .short 0x0009
    .short 13         # 13 functions
    .int 0, 0
    .int sys_net_name
    .int sys_net_fnid_table
    .int sys_net_fstub_table
    .int 0, 0, 0, 0

# -----------------------------------------------------------------------------
# Trampoline Functions & ELFv1 OPD Descriptors
# -----------------------------------------------------------------------------
.section ".sceStub.text","ax"
.align 2

.macro DEFINE_STUB name
.align 2
.globl __\name
__\name:
    mflr    r0
    std     r0, 16(r1)
    std     r2, 40(r1)
    stdu    r1, -128(r1)
    lis     r12, \name\()_stub@ha
    lwz     r12, \name\()_stub@l(r12)   # Load pointer to runtime descriptor
    lwz     r0, 0(r12)                  # Load resolved function entry address
    lwz     r2, 4(r12)                  # Load resolved SPRX TOC base
    mtctr   r0
    bctrl                               # Call into SPRX!
    addi    r1, r1, 128
    ld      r2, 40(r1)                  # Restore our application TOC (r2)
    ld      r0, 16(r1)
    mtlr    r0
    blr

.section ".opd","aw"
.align 3
.globl \name
\name:
    .quad __\name, .TOC.@tocbase, 0
.section ".sceStub.text","ax"
.endm

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
