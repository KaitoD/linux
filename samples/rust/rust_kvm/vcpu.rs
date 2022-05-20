// SPDX-License-Identifier: GPL-2.0
use kernel::bindings;
use kernel::pages::Pages;
use kernel::prelude::*;
use kernel::sync::{Mutex, Ref, UniqueRef};
use kernel::Result;
use core::arch::global_asm;
//use alloc::alloc::{AllocError};
use super::Guest;
use crate::exit::*;
use crate::mmu::*;
use crate::vmcs::*;
use crate::vmstat::*;
use core::ptr::NonNull;

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmRun {
    /* in */
    pub(crate) request_interrupt_window: u8,
    pub(crate) immediate_exit: u8,
    pub(crate) padding1: u32,
    pub(crate) padding2: u16,
    /* out */
    pub(crate) exit_reason: u32,
    pub(crate) ready_for_interrupt_injection: u8,
    pub(crate) if_flag: u8,
    pub(crate) flags: u16,
}

#[allow(dead_code)]
pub(crate) struct Vcpu {
    pub(crate) guest: Ref<Mutex<Guest>>,
    pub(crate) vmx_state: Box<VmxState>,
    // DefMut trait for UniqueRef, List use it
    pub(crate) mmu: UniqueRef<RkvmMmu>,
    pub(crate) va_run: u64,
    pub(crate) run: *mut RkvmRun,
    pub(crate) vcpu_id: u32,
    pub(crate) launched: bool,
}

impl Vcpu {
    pub(crate) fn new(guest: Ref<Mutex<Guest>>) -> Result<Ref<Mutex<Self>>> {
        let state = Box::try_new(VmxState::new()?);
        let state = match state {
            Ok(state) => state,
            Err(_) => return Err(Error::ENOMEM),
        };
        let page = Pages::<0>::new();
        let run = match page {
            Ok(page) => page,
            Err(err) => return Err(err),
        };
        let mut va = unsafe { bindings::rkvm_page_address(run.pages) };
        let mut mmu = RkvmMmu::new()?;
        let ptr =  NonNull::new(va as *mut RkvmRun).unwrap().as_ptr();
        mmu.init_mmu_root();
        let vcpu = unsafe {
            Ref::try_new(Mutex::new(Self {
                guest: guest,
                vmx_state: state,
                mmu: mmu,
                va_run: va,
                run: ptr,
                vcpu_id: 0,
                launched: false,
            }))?
        };
        Ok(vcpu)
    }

    pub(crate) fn init(&self, vmcsconf: &VmcsConfig) {
        vmcsconf.vcpu_vmcs_init();
    }

    pub(crate) fn get_run(&self) -> u64 {
        self.va_run
    }

    pub(crate) fn vcpu_exit_handler(&mut self) -> Result {
        let exit_info = ExitInfo::from_vmcs();
        unsafe { (*self.run).exit_reason = exit_info.exit_reason as u32;}
        match exit_info.exit_reason {
            ExitReason::HLT => handle_hlt(&exit_info, self),
            //ExitReason::IO_INSTRUCTION => handle_io_instruction(&exit_info),
            ExitReason::EPT_VIOLATION => handle_ept_violation(&exit_info, self),
            _ => Err(Error::ENOSPC),
        };
        Ok(())
    }

    pub(crate) fn vcpu_run(&mut self) -> i64 {
        loop {
            unsafe {
                bindings::rkvm_irq_disable();
            }
            let has_err_ = unsafe { _vmx_vcpu_run(&mut self.vmx_state, self.launched) };

            if has_err_ == 1 {
                unsafe {
                    bindings::rkvm_irq_enable();
                }
                return -1;
            }
            unsafe {
                bindings::rkvm_irq_enable();
            }

            //match vmexit_handler
            let ret = self.vcpu_exit_handler();
            //according to ret, update run
        } // loop
    }
}
/*
fn vmx_update_host(vmx_state: &mut VmxState, host_rsp: u64) {
    vmx_state.host_state.rsp = host_rsp;
    vmcs_write64(VmcsField::HOST_RSP, host_rsp);
}
*/
extern "C" {
    fn _vmx_vcpu_run(vmx_state: &mut VmxState, launched: bool) -> u64;
}

global_asm!(
    "
.global _vmx_vcpu_run
_vmx_vcpu_run:
    push   rbp
    mov    rbp,rsp
    push   r15
    push   r14
    push   r13
    push   r12
    push   rbx
    push   rdi
    mov    rsi,rbx
//    lea    rsi, -0x8[rsp]
    mov    rax,[rsp]
    test   bl,bl
    mov    rcx,0x8[rax]
    mov    rdx,0x10[rax]
    mov    rbx,0x18[rax]
    mov    rbp,0x28[rax]
    mov    rsi,0x30[rax]
    mov    rdi,0x38[rax]
    mov    r8,0x40[rax]
    mov    r9,0x48[rax]
    mov    r10,0x50[rax]
    mov    r11,0x58[rax]
    mov    r12,0x60[rax]
    mov    r13,0x68[rax]
    mov    r14,0x70[rax]
    mov    r15,0x78[rax]
    mov    rax,[rax]
    je 3f
   vmresume
    jmp 4f
3: vmlaunch
4:
    jbe    2f
    push   rax
    mov    rax,0x8[rsp]
//  pop   [rax]
    pop    rcx
    mov    [rax],rcx
    mov    0x8[rax],rcx
    mov    0x10[rax],rdx
    mov    0x18[rax],rbx
    mov    0x28[rax],rbp
    mov    0x30[rax],rsi
    mov    0x38[rax],rdi
    mov    0x40[rax],r8
    mov    0x48[rax],r9
    mov    0x50[rax],r10
    mov    0x58[rax],r11
    mov    0x60[rax],r12
    mov    0x68[rax],r13
    mov    0x70[rax],r14
    mov    0x78[rax],r15
    xor    eax,eax
1:  xor    ecx,ecx
    xor    edx,edx
    xor    ebx,ebx
    xor    ebp,ebp
    xor    esi,esi
    xor    edi,edi
    xor    r8d,r8d
    xor    r9d,r9d
    xor    r10d,r10d
    xor    r11d,r11d
    xor    r12d,r12d
    xor    r13d,r13d
    xor    r14d,r14d
    xor    r15d,r15d
    add    rsp, 0x8
    pop    rbx
    pop    r12
    pop    r13
    pop    r14
    pop    r15
    pop    rbp
    ret
2:  mov    eax, 0x1
    jmp    1b
"
);