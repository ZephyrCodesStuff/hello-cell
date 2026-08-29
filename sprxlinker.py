import sys
import struct

def process_elf(elf_path):
    with open(elf_path, 'r+b') as f:
        # 1. Parse ELF64 Header
        f.seek(0)
        ehdr = f.read(64)
        e_shoff, = struct.unpack('>Q', ehdr[40:48])
        e_shentsize, e_shnum, e_shstrndx = struct.unpack('>HHH', ehdr[58:64])
        
        shdrs = []
        for i in range(e_shnum):
            f.seek(e_shoff + i * e_shentsize)
            data = f.read(64)
            sh_name, sh_type, sh_flags, sh_addr, sh_offset, sh_size, sh_link, sh_info, sh_addralign, sh_entsize = struct.unpack('>IIQQQQIIQQ', data)
            shdrs.append({
                'name_idx': sh_name,
                'type': sh_type,
                'flags': sh_flags,
                'addr': sh_addr,
                'offset': sh_offset,
                'size': sh_size,
                'entsize': sh_entsize,
            })
        
        # Section name table
        shstr = shdrs[e_shstrndx]
        f.seek(shstr['offset'])
        shstrtab = f.read(shstr['size'])
        
        def get_name(idx):
            end = shstrtab.find(b'\x00', idx)
            return shstrtab[idx:end].decode('latin1')
        
        sections = {get_name(s['name_idx']): s for s in shdrs}
        
        # 3. Process .lib.stub imports count
        if '.lib.stub' in sections and '.rodata.sceFNID' in sections:
            stub_sec = sections['.lib.stub']
            fnid_sec = sections['.rodata.sceFNID']
            stub_count = stub_sec['size'] // 44
            
            stubs = []
            for i in range(stub_count):
                f.seek(stub_sec['offset'] + i * 44)
                h1, h2, imports, z1, z2, name_ptr, fnid_ptr, fstub_ptr = struct.unpack('>IHHIIIII', f.read(28))
                stubs.append((i, fnid_ptr))
            
            for i, fnid_ptr in stubs:
                end = fnid_sec['addr'] + fnid_sec['size']
                for j, other_fnid in stubs:
                    if i != j and other_fnid >= fnid_ptr and other_fnid < end:
                        end = other_fnid
                fnid_count = (end - fnid_ptr) // 4
                f.seek(stub_sec['offset'] + i * 44 + 6) # offset of 'imports' uint16
                f.write(struct.pack('>H', fnid_count))
        
        # 4. Pack .opd descriptors
        if '.opd' in sections:
            opd_sec = sections['.opd']
            count = opd_sec['size'] // 24
            for i in range(count):
                f.seek(opd_sec['offset'] + i * 24)
                func, rtoc, _ = struct.unpack('>QQQ', f.read(24))
                data = ((func << 32) | (rtoc & 0xFFFFFFFF)) & 0xFFFFFFFFFFFFFFFF
                f.seek(opd_sec['offset'] + i * 24 + 16)
                f.write(struct.pack('>Q', data))
        
        print(f"[sprxlinker] Processed {elf_path} successfully.")

if __name__ == '__main__':
    if len(sys.argv) > 1:
        process_elf(sys.argv[1])
    else:
        print("Usage: sprxlinker.py <elf_path>")
