// SPDX-License-Identifier: GPL-2.0
//use kernel::prelude::*;

#[allow(dead_code)]
pub(crate) struct HostState {
  // Host stack pointer.
  pub(crate) rsp: u64,

  // Extended control registers.
  pub(crate) xcr0: u64,
}

#[allow(dead_code)]
#[derive(Default)]
pub(crate) struct GuestState {
  //  RIP, RSP, and RFLAGS are automatically saved by VMX in the VMCS.
  pub(crate)  rax: u64,
  pub(crate)  rcx: u64,
  pub(crate)  rdx: u64,
  pub(crate)  rbx: u64,
  pub(crate)  rbp: u64,
  pub(crate)  rsi: u64,
  pub(crate)  rdi: u64,
  pub(crate)  r8: u64,
  pub(crate)  r9: u64,
  pub(crate)  r10: u64,
  pub(crate)  r11: u64,
  pub(crate)  r12: u64,
  pub(crate)  r13: u64,
  pub(crate)  r14: u64,
  pub(crate)  r15: u64,

  // Control registers.
  pub(crate)  cr2: u64,

  // Extended control registers.
  pub(crate)  xcr0: u64,
}

macro_rules! ONE {
     ($x: expr) => {
        (1 + (($x) - ($x)))
     }
}
macro_rules! BITS_SHIFT {
     ($x:expr, $high:expr, $low:expr) => {
        ((($x) >> ($low)) & ((ONE!($x) << (($high) - ($low) + 1)) - 1))
     }
}

#[allow(dead_code)]
impl GuestState {
  // Convenience getters for accessing low 32-bits of common registers.
  pub(crate) fn get_eax(&self) -> u32 { return self.rax as u32; }
  pub(crate) fn get_ecx(&self) -> u32 { return self.rcx as u32; }
  pub(crate) fn get_edx(&self) -> u32 { return self.rdx as u32; }
  pub(crate) fn get_ebx(&self) -> u32 { return self.rbx as u32; }

  // Convenience getter/setter for fetching the 64-bit value edx:eax, used by
  // several x86_64 instructions, such as `rdmsr` and `wrmsr`.
  //
  // For reads, the top bits of rax and rdx are ignored (c.f. Volume 2C,
  // WRMSR). For writes, the top bits of rax and rdx are set to zero, matching
  // the behaviour of x86_64 instructions such as `rdmsr` (c.f. Volume 2C,
  // RDMSR).

  pub(crate) fn get_edx_eax(&self) -> u64 { return (self.get_edx() as u64) << 32 | (self.get_eax() as u64); }
  pub(crate) fn set_edx_eax(&mut self, value: u64) {
    self.rax = BITS_SHIFT!(value, 31, 0);
    self.rdx = BITS_SHIFT!(value, 63, 32);
  }
}

