// SPDX-License-Identifier: GPL-2.0
//use kernel::bit;

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

pub(crate) struct RFlags {
    pub(crate) value: u64,
}

#[allow(dead_code)]
impl RFlags {
    pub(crate) const FLAGS_IF: u64 = 9;
    pub(crate) const FLAGS_ZF: u64 = 6;
    pub(crate) const FLAGS_AF: u64 = 4;
    pub(crate) const FLAGS_A1: u64 = 1;
    pub(crate) const FLAGS_CF: u64 = 0;
}

#[repr(u32)]
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub(crate) enum X86Msr {
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
