// SPDX-License-Identifier: GPL-2.0
use kernel::bindings;
use kernel::pages::Pages;
#[feature(global_asm)]
use kernel::prelude::*;
use kernel::sync::{Mutex, Ref, UniqueRef};
use kernel::{Result, PAGE_SIZE};
use kernel::c_types::c_void;
use core::pin::Pin;
//use alloc::alloc::{AllocError};
use super::{Guest, GuestWrapper};
use crate::exit::*;
use crate::mmu::*;
use crate::vmcs::*;
use crate::vmstat::*;
use core::ptr::NonNull;

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmRegs {
        /* out (KVM_GET_REGS) / in (KVM_SET_REGS) */
        pub(crate) rax: u64, 
        pub(crate) rbx: u64, 
        pub(crate) rcx: u64,
        pub(crate) rdx: u64,
        pub(crate) rsi: u64, 
        pub(crate) rdi: u64,
        pub(crate) rsp: u64,
        pub(crate) rbp: u64,
        pub(crate) r8:  u64,
        pub(crate) r9: u64,
        pub(crate) r10: u64,
        pub(crate) r11: u64,
        pub(crate) r12: u64,
        pub(crate) r13: u64,
        pub(crate) r14: u64,
        pub(crate) r15: u64,
        pub(crate) rip: u64,
        pub(crate) rflags: u64,
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmSegment {
	pub(crate) base: u64,
	pub(crate) limit: u32,
	pub(crate) selector: u16,
	pub(crate) rtype: u8,
	pub(crate) present: u8,
        /* dpl, db, s, l, g, avl, unusable,padding*/
        pub(crate) padding: u64,
}

impl RkvmSegment {
   pub(crate) fn new() -> Self {
         Self {
              base: 0,
              limit: 0,
              selector: 0,
              rtype: 0,
              present: 0,
              padding: 0,
         }
   }
}
   
#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmDtable {
	pub(crate) base: u64,
	pub(crate) limit: u16,
	pub(crate) padding: [u16;3],
}

impl RkvmDtable {
   pub(crate) fn new() -> Self {
       Self {
          base: 0,
          limit: 0,
          padding: [0, 0, 0,],
       }
   }
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmSregs {
	/* out (KVM_GET_SREGS) / in (KVM_SET_SREGS) */
	pub(crate) cs: RkvmSegment,
        pub(crate) ds: RkvmSegment,
        pub(crate) es: RkvmSegment, 
        pub(crate) fs: RkvmSegment,
        pub(crate) gs: RkvmSegment, 
        pub(crate) ss: RkvmSegment,
	pub(crate) tr: RkvmSegment,
        pub(crate) ldt: RkvmSegment,
	pub(crate) gdt: RkvmDtable,
        pub(crate) idt: RkvmDtable,
	pub(crate) cr0: u64,
        pub(crate) cr2: u64,
        pub(crate) cr3: u64,
        pub(crate) cr4: u64,
        pub(crate) cr8: u64,
	pub(crate) efer: u64,
        pub(crate) apic_base: u64,
        pub(crate) interrupt_bitmap: [u64;4],
}

impl RkvmSregs {
    pub(crate) fn new() -> Self {
       Self {
         cs: RkvmSegment::new(),
         ds: RkvmSegment::new(),
         es: RkvmSegment::new(),
         fs: RkvmSegment::new(),
         gs: RkvmSegment::new(),
         ss: RkvmSegment::new(),
         tr: RkvmSegment::new(),
         ldt: RkvmSegment::new(),
         gdt: RkvmDtable::new(),
         idt: RkvmDtable::new(),
         cr0: 0,
         cr2: 0,
         cr3: 0,
         cr4: 0,
         cr8: 0,
         efer: 0,
         apic_base: 0,
         interrupt_bitmap: [0,0,0,0],
       }
    }
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct Pio {
   pub(crate) direction: u8,
   pub(crate) size: u8,
   pub(crate) port: u16,
   pub(crate) count: u32,
   pub(crate) data_offset: u64,
}
        
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
    pub(crate) cr8: u64,
    pub(crate) apic_base: u64,
    pub(crate) io: Pio, 
}


#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmVmcs {
   pub(crate) revision_id: u32,
   pub(crate) abort: u32,
}

/*
#[repr(C)]
#[allow(dead_code)]
struct VmxInfo {
    revision_id: u32,
    region_size: u16,
    write_back: bool,
    io_exit_info: bool,
    vmx_controls: bool,
}
*/

#[allow(dead_code)]
pub(crate) struct RkvmPage<T> {
    pub(crate) rpage: Pages<0>,
    pub(crate) va: u64,
    pub(crate) ptr: *mut T,
}

impl<T>  RkvmPage<T> {
    pub(crate) fn new(rpage: Pages<0>) -> Self {
       let va = unsafe { bindings::rkvm_page_address(rpage.pages) };
       let ptr = va as *mut c_void;
       unsafe { bindings::memset(ptr, 0, PAGE_SIZE as u64); }
       let ptr = va as *mut T;//NonNull::new(va as *mut T).unwrap().as_ptr();

       Self {
          rpage: rpage,
          va: va,
          ptr: ptr,
       }
    }
}

#[allow(dead_code)]
pub(crate) struct Vcpu {
    pub(crate) guest: Ref<GuestWrapper>,
    pub(crate) guest_state: Pin<Box<GuestState>>,
    // DefMut trait for UniqueRef, List use it
    pub(crate) mmu: UniqueRef<RkvmMmu>,
    pub(crate) run: RkvmPage<RkvmRun>,
    pub(crate) vmcs: RkvmPage<RkvmVmcs>,
    pub(crate) vcpu_id: u32,
    pub(crate) launched: bool,
}

pub(crate) fn alloc_vmcs(revision_id: u32) -> Result<RkvmPage<RkvmVmcs>> {
    let page = Pages::<0>::new();
    let page = match page {
            Ok(page) => page,
            Err(err) => return Err(err),
     };
     let mut vmcs = RkvmPage::<RkvmVmcs>::new(page);
     //unsafe { (*vmcs.ptr).revision_id = revision_id; }
     let ptr = vmcs.va as *mut u32;
     unsafe { *ptr = revision_id; }
     pr_info!(
            "Rust kvm: vmcs={:x},revision={:?} \n",
            vmcs.va,
            *ptr,
      );
     Ok(vmcs)
}

fn vmcs_load(va: u64) {
   unsafe {
      let phy = bindings::rkvm_phy_address(va);
      if phy == 0 {
         pr_info!(" vmcs_load failed \n");
      }
      bindings::rkvm_vmcs_load(phy);
   }
}

fn vmcs_clear(va: u64) {
   unsafe {
       let phy = bindings::rkvm_phy_address(va);
       bindings::rkvm_vmcs_clear(phy);
   }
}


pub(crate) struct VcpuWrapper {
    pub(crate) vcpuinner: Mutex<Vcpu>,
}
impl VcpuWrapper {
    pub(crate) fn new(guest: Ref<GuestWrapper>, revision_id: u32) -> Result<Ref<Self>> {
        let state = Pin::from(Box::try_new(GuestState::new())?);
       /* let state = match state {
            Ok(state) => state,
            Err(_) => return Err(Error::ENOMEM),
        };*/
        // kvm_run
        let page = Pages::<0>::new();
        let run = match page {
            Ok(page) => page,
            Err(err) => return Err(err),
        };
        let run = RkvmPage::<RkvmRun>::new(run);
        // alloc vmcs and init 
        let vmcs = alloc_vmcs(revision_id);
        let vmcs = match vmcs {
            Ok(vmcs) => vmcs,
            Err(err) => return Err(err),
        };
        //vmcs_clear(vmcs.va);

        let mmu = RkvmMmu::new();

        let mut mmu = match mmu {
            Ok(mmu) => mmu,
            Err(err) => return Err(err),
        };
        //vmcs_load(vmcs.va);
        //mmu.init_mmu_root();
        let mut v = Pin::from(UniqueRef::try_new(Self {
            vcpuinner: unsafe {
                Mutex::new(Vcpu {
                    guest: guest,
                    guest_state:  state,
                    mmu: mmu,
                    run: run,
                    vmcs: vmcs,
                    vcpu_id: 0,
                    launched: false,
                })
            },
        })?);
        let pinned = unsafe { v.as_mut().map_unchecked_mut(|s| &mut s.vcpuinner) };
        kernel::mutex_init!(pinned, "VcpuWrapper::vcpuinner");

        Ok(v.into())
    }

    pub(crate) fn init(&self, vmcsconf: &mut VmcsConfig) {
        let mut vcpuinner = self.vcpuinner.lock();
        vmcs_clear(vcpuinner.vmcs.va);
        vmcs_load(vcpuinner.vmcs.va);
        vmcsconf.vcpu_vmcs_init();
        vcpuinner.mmu.init_mmu_root();
    }

    pub(crate) fn get_run(&self) -> u64 {
        self.vcpuinner.lock().run.va
    }

    pub(crate) fn vcpu_exit_handler(&self) -> Result<u64> {
        let exit_info = ExitInfo::from_vmcs();
        //let mut vcpuinner = self.vcpuinner.lock();
        //unsafe {
          //  (*vcpuinner.run).exit_reason = exit_info.exit_reason as u32;
       // }

        match exit_info.exit_reason {
            ExitReason::HLT => return handle_hlt(&exit_info, self),
            ExitReason::IO_INSTRUCTION => return handle_io(&exit_info, self),
            ExitReason::EPT_VIOLATION => return handle_ept_violation(&exit_info, self),
            ExitReason::EXTERNAL_INTERRUPT => return Ok(1),
            _ => {  pr_info!(" ## exit_reason = {:?} \n", exit_info.exit_reason);
                    let mut vcpuinner = self.vcpuinner.lock();
                    let ptr = (vcpuinner.run.va + 8) as *mut u64;
                    unsafe {
                        (*ptr) = exit_info.exit_reason as u64;
                        pr_info!(" ## exit_reason = {:x} \n", *ptr);
                     }
                    return Err(Error::EINVAL);
                 }
        };
    }

    pub(crate) fn vcpu_run(&self) -> i64 {
        {
          let mut vcpuinner = self.vcpuinner.lock();
           vmcs_load(vcpuinner.vmcs.va);
         
           vmcs_write64(VmcsField::GUEST_RIP, vcpuinner.guest_state.rip);
           let rip = vmcs_read64(VmcsField::GUEST_RIP);
           let state_host_rsp = vcpuinner.guest_state.as_ref().get_ref() as *const _ as u64;
           pr_info!(" vcpu_run state_rip = {:x}, state_rax ={:x}, guest_rip={:x}, host_rsp = {:x}\n", 
                                  vcpuinner.guest_state.rip,
                                  vcpuinner.guest_state.rax, rip, state_host_rsp);
          vmcs_write64(
            VmcsField::HOST_RSP,
            vcpuinner.guest_state.as_ref().get_ref() as *const _ as u64,
          );
          
        }
        loop {
            let exit_reason: u32 = 0;
            unsafe {
                bindings::rkvm_irq_disable();
            }
            let has_err_;
            { 
                let mut vcpuinner = self.vcpuinner.lock();
                let launched = vcpuinner.guest_state.launched;
                
                pr_info!(" vmentry: launched = {:?}, guest_rip={:x} \n", launched, vmcs_read64(VmcsField::GUEST_RIP)); 
                has_err_ = unsafe { _vmx_vcpu_run(&vcpuinner.guest_state) };
            }
               
               pr_info!(" vmexit: guest_rip={:x} \n", vmcs_read64(VmcsField::GUEST_RIP));
            if has_err_ == 1 {
                unsafe {
                    bindings::rkvm_irq_enable();
                }
                let mut vcpuinner = self.vcpuinner.lock();
                //unsafe { (*vcpuinner.run.ptr).exit_reason = 0xdead;}
                //let ptr = NonNull::new(vcpuinner.run.va as *mut c_void).unwrap().as_ptr();
                //unsafe { bindings::memset(ptr, 0xff, PAGE_SIZE as u64); }
                //let ptr = NonNull::new(vcpuinner.run.va as *mut RkvmRun).unwrap().as_ptr();
                let ptr = (vcpuinner.run.va + 8) as *mut u64;
                dump_vmcs(); 
                let host_rsp = vmcs_read64(VmcsField::HOST_RSP);
                unsafe { (*ptr)= 0xdead;
                         (*(vcpuinner.run.ptr)).exit_reason = 0xffff;
                }
                pr_info!( " run = {:x}, host_rsp {:x} \n", *ptr, host_rsp);
                //vmcs_load(vcpuinner.vmcs.va);
                let ret = vmcs_read32(VmcsField::VM_INSTRUCTION_ERROR);
                let rflags = unsafe { bindings::rkvm_rflags_read()};
                pr_info!("run loop after _vmx_vcpu_run, rflags={:x},ret={:x} \n", rflags, ret);
                return -1;
            }
            unsafe {
                bindings::rkvm_irq_enable();
            }
            { 
              let mut vcpuinner = self.vcpuinner.lock(); 
              vcpuinner.guest_state.launched = true;
            }
            //match vmexit_handler
            let ret = self.vcpu_exit_handler();
            pr_info!("ret={:?}, after vcpu_exit_handler \n", ret);
            // TODO: according to ret, update run
            match ret {
                Ok(r) => {
                    if r == 0 {
                        return r.try_into().unwrap();
                    }
                }
                Err(err) => {
                         let mut vcpuinner = self.vcpuinner.lock();
                         let ptr = (vcpuinner.run.va + 8) as *mut u64;
                         pr_info!("  vcpu run failed \n");
                         unsafe { (*ptr)= 9 };
                         return -1;
                }
            }
        } // loop
    }
   pub(crate) fn set_regs(&self, regs: &RkvmRegs)  {
       let mut vcpuinner = self.vcpuinner.lock();
       vmcs_load(vcpuinner.vmcs.va);
       
       vcpuinner.guest_state.rax = regs.rax;
       vcpuinner.guest_state.rbx = regs.rbx;
       vcpuinner.guest_state.rcx = regs.rcx;
       vcpuinner.guest_state.rdx = regs.rdx;
       vcpuinner.guest_state.rsi = regs.rsi;
       vcpuinner.guest_state.rdi = regs.rdi;
       vcpuinner.guest_state.rsp = regs.rsp;
       vcpuinner.guest_state.rbp = regs.rbp;
       vcpuinner.guest_state.r8  = regs.r8;
       vcpuinner.guest_state.r9  = regs.r9;
       vcpuinner.guest_state.r10 = regs.r10;
       vcpuinner.guest_state.r11 = regs.r11;
       vcpuinner.guest_state.r12 = regs.r12;
       vcpuinner.guest_state.r13 = regs.r13;
       vcpuinner.guest_state.r14 = regs.r14;
       vcpuinner.guest_state.r15 = regs.r15;
       vcpuinner.guest_state.rip = regs.rip;
       vmcs_write64(VmcsField::GUEST_RIP, regs.rip);
       let rip = vmcs_read64(VmcsField::GUEST_RIP);
       pr_info!(" set_regs guest_rip = {:x}, state_rax = {:x}, read_rip={:x} \n", regs.rip, vcpuinner.guest_state.rax, rip);
       vmcs_write64(VmcsField::GUEST_RFLAGS, regs.rflags);
   }
   
   pub(crate) fn get_regs(&self, regs: &mut RkvmRegs)  {
       let mut vcpuinner = self.vcpuinner.lock();
       vmcs_load(vcpuinner.vmcs.va);
       //let guest_state = &vcpuinner.guest_state;
       regs.rax = vcpuinner.guest_state.rax;
       regs.rbx = vcpuinner.guest_state.rbx;
       regs.rcx = vcpuinner.guest_state.rcx;
       regs.rdx = vcpuinner.guest_state.rdx;
       regs.rsi = vcpuinner.guest_state.rsi;
       regs.rdi = vcpuinner.guest_state.rdi;
       regs.rsp = vcpuinner.guest_state.rsp;
       regs.rbp = vcpuinner.guest_state.rbp;
       regs.r8  = vcpuinner.guest_state.r8;
       regs.r9  = vcpuinner.guest_state.r9;
       regs.r10 = vcpuinner.guest_state.r10;
       regs.r11 = vcpuinner.guest_state.r11;
       regs.r12 = vcpuinner.guest_state.r12;
       regs.r13 = vcpuinner.guest_state.r13;
       regs.r14 = vcpuinner.guest_state.r14;
       regs.r15 = vcpuinner.guest_state.r15;
       regs.rip = vcpuinner.guest_state.rip;
       regs.rflags = vmcs_read64(VmcsField::GUEST_RFLAGS);       
   }

   pub(crate) fn set_sregs(&self, sregs: &RkvmSregs)  {
       let mut vcpuinner = self.vcpuinner.lock();
       vmcs_load(vcpuinner.vmcs.va);
       let base = sregs.cs.base;
       let selector = sregs.cs.selector;
       vmcs_write64(VmcsField::GUEST_CS_BASE, base);
       vmcs_write16(VmcsField::GUEST_CS_SELECTOR, selector);
       
   }

   pub(crate) fn get_sregs(&self, sregs: &mut RkvmSregs)  {
      let mut vcpuinner = self.vcpuinner.lock();
      vmcs_load(vcpuinner.vmcs.va);
      sregs.cs.base = vmcs_read64(VmcsField::GUEST_CS_BASE);
      sregs.cs.selector = vmcs_read16(VmcsField::GUEST_CS_SELECTOR);
   }
}

extern "C" {
    fn _vmx_vcpu_run(guest_state: &GuestState) -> u32;
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
    
    mov     [rdi], rsp
    mov     rsp, rdi

//    test   bl,bl
    add     rsp, 8
    pop     rax
    pop     rcx
    pop     rdx
    pop     rbx
    add     rsp, 8 // skip rsp
    pop     rbp
    pop     rsi
    pop     rdi
    pop     r8
    pop     r9
    pop     r10
    pop     r11
    pop     r12
    pop     r13
    pop     r14
    pop     r15

    cmp     byte ptr [rsp], 0
    je 3f
    vmresume
    jmp 4f
3:  vmlaunch

4:
    // We will only be here if vmlaunch or vmresume failed.
    // Restore host callee, RSP and return address.
    mov     rsp, [rsp - 17*8]
    pop     rbx
    pop     r12
    pop     r13
    pop     r14
    pop     r15
    pop     rbp

    // return true
    mov     eax, 1
    ret
"
);

