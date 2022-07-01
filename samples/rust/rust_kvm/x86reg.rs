// SPDX-License-Identifier: GPL-2.0

//control registers
#[allow(dead_code)]
pub(crate) struct Cr0 {
    pub(crate) value: usize,
}

#[allow(dead_code)]
impl Cr0 {
    pub(crate) const CR0_ENABLE_PAGING: usize = 31;
    pub(crate) const CR0_TS: usize = 1 << 3;
}

#[allow(dead_code)]
pub(crate) struct Cr4 {
    pub(crate) value: usize,
}

impl Cr4 {
    pub(crate) const CR4_ENABLE_VMX: usize = 13;
}

#[repr(u64)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub(crate) enum RFlags {
    FLAGS_IF = 1 << 9,
    FLAGS_ZF = 1 << 6,
    FLAGS_AF = 1 << 4,
    FLAGS_A1 = 1 << 1,
    FLAGS_CF = 1 << 0,
}

#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum X86Msr {
    VMX_BASIC = 0x00000480,
    FS_BASE = 0xc0000100, /* fs base address */
    GS_BASE = 0xc0000101, /* gs base address */
    KERNEL_GS_BASE = 0xc0000102,
    PAT = 0x00000277,
    EFER = 0xc0000080,
    PINBASED_CTLS = 0x0481,
    PROCBASED_CTLS = 0x0482,
    EXIT_CTLS = 0x0483,
    ENTRY_CTLS = 0x0484,
    PROCBASED_CTLS2 = 0x048b,
    VMX_EPT_VPID_CAP = 0x048c,
    TRUE_PINBASED_CTLS = 0x048d,
    TRUE_PROCBASED_CTLS = 0x048e,
    TRUE_EXIT_CTLS = 0x048f,
    TRUE_ENTRY_CTLS = 0x490,
}
