// SPDX-License-Identifier: GPL-2.0
use super::{Guest, GuestWrapper};
use super::{Vcpu, VcpuWrapper};
use crate::mmu::*;
use crate::vmcs::*;
use core::mem::MaybeUninit;
use kernel::prelude::*;
use kernel::{bindings, bit, c_types::c_void, pages::Pages, sync::Ref, Error, Result, PAGE_SIZE};

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub(crate) enum ExitReason {
    EXCEPTION_OR_NMI = 0,
    EXTERNAL_INTERRUPT = 1,
    TRIPLE_FAULT = 2,
    INIT_SIGNAL = 3,
    STARTUP_IPI = 4,
    IO_SMI = 5,
    OTHER_SMI = 6,
    INTERRUPT_WINDOW = 7,
    NMI_WINDOW = 8,
    TASK_SWITCH = 9,
    CPUID = 10,
    GETSEC = 11,
    HLT = 12,
    INVD = 13,
    INVLPG = 14,
    RDPMC = 15,
    RDTSC = 16,
    RSM = 17,
    VMCALL = 18,
    VMCLEAR = 19,
    VMLAUNCH = 20,
    VMPTRLD = 21,
    VMPTRST = 22,
    VMREAD = 23,
    VMRESUME = 24,
    VMWRITE = 25,
    VMXOFF = 26,
    VMXON = 27,
    CONTROL_REGISTER_ACCESS = 28,
    MOV_DR = 29,
    IO_INSTRUCTION = 30,
    RDMSR = 31,
    WRMSR = 32,
    ENTRY_FAILURE_GUEST_STATE = 33,
    ENTRY_FAILURE_MSR_LOADING = 34,
    MWAIT = 36,
    MONITOR_TRAP_FLAG = 37,
    MONITOR = 39,
    PAUSE = 40,
    ENTRY_FAILURE_MACHINE_CHECK = 41,
    TPR_BELOW_THRESHOLD = 43,
    APIC_ACCESS = 44,
    VIRTUALIZED_EOI = 45,
    ACCESS_GDTR_OR_IDTR = 46,
    ACCESS_LDTR_OR_TR = 47,
    EPT_VIOLATION = 48,
    EPT_MISCONFIGURATION = 49,
    INVEPT = 50,
    RDTSCP = 51,
    VMX_PREEMPT_TIMER_EXPIRED = 52,
    INVVPID = 53,
    WBINVD = 54,
    XSETBV = 55,
    APIC_WRITE = 56,
    RDRAND = 57,
    INVPCID = 58,
    VMFUNC = 59,
    ENCLS = 60,
    RDSEED = 61,
    PAGE_MODIFICATION_LOG_FULL = 62,
    XSAVES = 63,
    XRSTORS = 64,
    SPP_EVENT = 66,
    UMWAIT = 67,
    TPAUSE = 68,
    BUSLOCK = 74,
    UNKNOWN = 200,
}

impl From<u32> for ExitReason {
    fn from(v: u32) -> Self {
        match v {
            0 => ExitReason::EXCEPTION_OR_NMI,
            1 => ExitReason::EXTERNAL_INTERRUPT,
            2 => ExitReason::TRIPLE_FAULT,
            3 => ExitReason::INIT_SIGNAL,
            4 => ExitReason::STARTUP_IPI,
            5 => ExitReason::IO_SMI,
            6 => ExitReason::OTHER_SMI,
            7 => ExitReason::INTERRUPT_WINDOW,
            8 => ExitReason::NMI_WINDOW,
            9 => ExitReason::TASK_SWITCH,
            10 => ExitReason::CPUID,
            11 => ExitReason::GETSEC,
            12 => ExitReason::HLT,
            13 => ExitReason::INVD,
            14 => ExitReason::INVLPG,
            15 => ExitReason::RDPMC,
            16 => ExitReason::RDTSC,
            17 => ExitReason::RSM,
            18 => ExitReason::VMCALL,
            28 => ExitReason::CONTROL_REGISTER_ACCESS,
            29 => ExitReason::MOV_DR,
            30 => ExitReason::IO_INSTRUCTION,
            31 => ExitReason::RDMSR,
            32 => ExitReason::WRMSR,
            33 => ExitReason::ENTRY_FAILURE_GUEST_STATE,
            34 => ExitReason::ENTRY_FAILURE_MSR_LOADING,
            36 => ExitReason::MWAIT,
            37 => ExitReason::MONITOR_TRAP_FLAG,
            39 => ExitReason::MONITOR,
            40 => ExitReason::PAUSE,
            41 => ExitReason::ENTRY_FAILURE_MACHINE_CHECK,
            43 => ExitReason::TPR_BELOW_THRESHOLD,
            44 => ExitReason::APIC_ACCESS,
            45 => ExitReason::VIRTUALIZED_EOI,
            46 => ExitReason::ACCESS_GDTR_OR_IDTR,
            47 => ExitReason::ACCESS_LDTR_OR_TR,
            48 => ExitReason::EPT_VIOLATION,
            49 => ExitReason::EPT_MISCONFIGURATION,
            50 => ExitReason::INVEPT,
            51 => ExitReason::RDTSCP,
            52 => ExitReason::VMX_PREEMPT_TIMER_EXPIRED,
            53 => ExitReason::INVVPID,
            54 => ExitReason::WBINVD,
            55 => ExitReason::XSETBV,
            56 => ExitReason::APIC_WRITE,
            57 => ExitReason::RDRAND,
            58 => ExitReason::INVPCID,
            59 => ExitReason::VMFUNC,
            60 => ExitReason::ENCLS,
            61 => ExitReason::RDSEED,
            62 => ExitReason::PAGE_MODIFICATION_LOG_FULL,
            _ => ExitReason::UNKNOWN,
        }
    }
}

#[repr(u64)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum PferrMask {
    PFERR_PRESENT_MASK = 1,
    PFERR_WRITE_MASK = 2,
    PFERR_USER_MASK = 4,
    PFERR_RSVD_MASK = 8,
    PFERR_FETCH_MASK = 16,
    PFERR_GUEST_FINAL_MASK = 1 << 32,
    PFERR_GUEST_PAGE_MASK = 1 << 33,
}

#[repr(u64)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum EptViolationMask {
    EPT_VIOLATION_ACC_READ = 1,
    EPT_VIOLATION_ACC_WRITE = 2,
    EPT_VIOLATION_ACC_INSTR = 4,
    EPT_VIOLATION_READABLE = 8,
    EPT_VIOLATION_WRITABLE = 16,
    EPT_VIOLATION_EXECUTABLE = 32,
    EPT_VIOLATION_GVA_TRANSLATED = 64,
}

#[derive(Debug)]
struct RkvmPageFault {
    addr: u64,
    error_code: u64,
    prefetch: bool,
    //get from error_code
    //exec: bool,
    //write: bool,
    present: bool,
    //rsvd: bool,
    //user: bool,
    gfn: u64,
    //output
    pfn: u64,
    hva: u64,
    goal_level: u64,
}

pub(crate) struct ExitInfo {
    pub(crate) entry_failure: bool,
    pub(crate) exit_reason: ExitReason,
    pub(crate) exit_instruction_length: u32,
    pub(crate) exit_qualification: u64,
    pub(crate) guest_rip: u64,
}

impl ExitInfo {
    pub(crate) fn from_vmcs() -> Self {
        let full_reason = vmcs_read32(VmcsField::VM_EXIT_REASON);
        Self {
            exit_reason: (full_reason & 0x1ffffu32).try_into().unwrap(),
            entry_failure: (full_reason & bit(31)) != 0,
            exit_instruction_length: vmcs_read32(VmcsField::VM_EXIT_INSTRUCTION_LEN),
            exit_qualification: vmcs_read64(VmcsField::EXIT_QUALIFICATION),
            guest_rip: vmcs_read64(VmcsField::GUEST_RIP),
        }
    }

    fn next_rip(&self) {
        vmcs_write64(
            VmcsField::GUEST_RIP,
            self.guest_rip + self.exit_instruction_length as u64,
        );
    }
}

pub(crate) fn handle_hlt(exit_info: &ExitInfo, vcpu: &VcpuWrapper) -> Result<u64> {
    pr_info!(" handle_hlt \n");
    let mut vcpuinner = vcpu.vcpuinner.lock();
    let ptr = (vcpuinner.run.va + 8) as *mut u64;
    unsafe {
             (*ptr) = 5; //exit_info.exit_reason as u32;
    }
    exit_info.next_rip();
    Ok(0)
}

pub(crate) fn handle_io(exit_info: &ExitInfo, vcpu: &VcpuWrapper) -> Result<u64> {
   pr_info!(" handle_io \n");
   let exit_qualification = exit_info.exit_qualification;
   let mut vcpuinner = vcpu.vcpuinner.lock();
   let ptr = (vcpuinner.run.va + 8) as *mut u64;
   unsafe {
       (*ptr) = 2;
       

//       (*vcpuinner.run.ptr).exit_reason = exit_info.exit_reason as u32;
       (*vcpuinner.run.ptr).io.port = (exit_qualification >> 16) as u16;
       (*vcpuinner.run.ptr).io.size = ((exit_qualification & 7) + 1) as u8;
       (*vcpuinner.run.ptr).io.direction = ((exit_qualification & 8) != 0) as u8;
       (*vcpuinner.run.ptr).io.count = 1;

   }
   exit_info.next_rip();
   Ok(0)
}
const LEVELBITS: u64 = 9;
macro_rules! LEVEL_SHIFT {
    ($level:expr) => {
        (bindings::PAGE_SHIFT as u64 + ($level - 1) * LEVELBITS)
    };
}

macro_rules! SHADOW_PT_INDEX {
    ($addr:expr, $level:expr) => {
        (($addr) >> (LEVEL_SHIFT!($level)) & ((1 << LEVELBITS) - 1))
    };
}

macro_rules! SPTE_TO_PFN {
    ($pte:expr) => {
       (($pte) & (bindings::physical_mask & ~((PAGE_SIZE - 1) as u64)))
    };
}
macro_rules! RKVM_PAGES_PER_HPAGE {
    ($level: expr) => {
        ((1 << ((($level) - 1) * 9 + bindings::PAGE_SHIFT as u64)) / PAGE_SIZE)
    };
}
fn rkvm_pagefault(vcpu: &VcpuWrapper, fault: &mut RkvmPageFault) -> Result {
    let vcpuinner = vcpu.vcpuinner.lock();
    let slot = &vcpuinner.guest.guestinner.lock().memslot;

    let uaddr = slot.userspace_addr;
    let base_gfn = slot.base_gfn;
    pr_info!(" ### rkvm_pagefault base_gfn={:x}, fault.gfn={:x} \n", base_gfn, fault.gfn);
    if fault.gfn < base_gfn {
       return Err(Error::EINVAL);
    } 
    fault.hva = uaddr + (fault.gfn - base_gfn) * (kernel::PAGE_SIZE as u64);
    let mut flags: u32 = bindings::FOLL_HWPOISON;

    let mut nrpages: i64 = 0;
    let mut pages = MaybeUninit::<*mut bindings::page>::uninit();
    unsafe {
        //let tmp: *mut *mut bindings::page
        nrpages = bindings::get_user_pages_unlocked(uaddr, 1, pages.as_mut_ptr(), flags);
        if nrpages != 1 {
            return Err(Error::ENOMEM);
        }
        //pages = pages.assume_init();
        let page = *pages.as_mut_ptr() as *mut bindings::page;
        fault.pfn = bindings::rkvm_page_to_pfn(page);
        fault.goal_level = 1;
    }
    pr_info!("pagefault: pfn={:x} \n", fault.pfn);
    Ok(())
}

fn rkvm_read_spte(mmu_page: Ref<RkvmMmuPage>, gfn: u64, level: u64) -> Result<u64> {
    if level < 1 {
       pr_info!(" rkvm_read_spte level={:?} < 1 \n", level);
       return Err(Error::EINVAL);
    }
    let offset: usize = SHADOW_PT_INDEX!((gfn >> bindings::PAGE_SHIFT), level) as usize;
    let mut spte: u64 = 0;
    let p = &mut spte;
    unsafe {
        let ptr = core::slice::from_raw_parts_mut((p as *mut u64) as *mut u8, 8);
        mmu_page.pages.read(ptr.as_mut_ptr(), offset, 8);
    }
    Ok(spte)
}

fn rkvm_write_spte(mmu_page: Ref<RkvmMmuPage>, new_spte: u64, gfn: u64, level: u64) -> Result {
    if level < 1 {
       pr_info!(" rkvm_write_spte level={:?} < 1 \n", level);
       return Err(Error::EINVAL);
    }
    let offset: usize = SHADOW_PT_INDEX!((gfn >> bindings::PAGE_SHIFT), level) as usize;
    let p = &new_spte;
    unsafe {
        let ptr = core::slice::from_raw_parts((p as *const u64) as *const u8, 8);
        mmu_page.pages.write(ptr.as_ptr(), offset, 8);
    }
    Ok(())
}

fn make_level_gfn(gfn: u64, level: u64) -> Result<u64> {
    if gfn == 0 {
       return Ok(0);
    }
    if level < 1 {
       pr_info!(" make_level_gfn: level={:?} < 1 \n", level);
       return Err(Error::EINVAL);
    }
    let level_gfn = gfn & (-1 * RKVM_PAGES_PER_HPAGE!(level) as i64) as u64;
    Ok(level_gfn)
}

fn make_spte(pfn: u64) -> u64 {
    let mut spte: u64 = 1u64 << 11; //SPTE_MMU_PRESENT_MASK
    let pa = pfn << bindings::PAGE_SHIFT;
    //TODO: permission settings in pte
    spte |= pa | 7u64 | 0x600000000000000;
    spte
}

fn make_noleaf_spte(pt: u64) -> u64 {
    let mut spte: u64 = 1u64 << 11; //SPTE_MMU_PRESENT_MASK
    let pa = unsafe { bindings::rkvm_phy_address(pt) };
    //TODO: permission settings in pte
    spte |= pa | 7u64;
    spte
}
fn rkvm_tdp_map(vcpu: &VcpuWrapper, fault: &mut RkvmPageFault) -> Result {
    let mut level: u64 = 4;
    let mut vcpuinner = vcpu.vcpuinner.lock();
    /*let mut pt_path = match vcpu.mmu.root_mmu_page.spt {
                          Some(spt) => spt,
                          None => return Err(Error::ENOMEM),
    };
    */

    let level_gfn = make_level_gfn(fault.gfn, level);
    let mut level_gfn = match level_gfn {
       Ok(gfn) => gfn,
       Err(e) => return Err(e),
    };
    let mut pre_mmu_page = vcpuinner.mmu.root_mmu_page.clone();
    let spte = rkvm_read_spte(pre_mmu_page.clone(), level_gfn, level);
    let mut spte = match spte {
      Err(err) => return Err(err),
      Ok(spte) => spte,
    };
    while level > 0 {
        if level == fault.goal_level {
            break;
        }
        if !vcpuinner.mmu.is_pte_present(spte) {
            let mut mmu_page = vcpuinner.mmu.alloc_mmu_page(level - 1, level_gfn)?;
            let child_spt = match mmu_page.spt {
                Some(spt) => spt,
                None => return Err(Error::ENOMEM),
            };
            spte = make_noleaf_spte(child_spt);
            rkvm_write_spte(pre_mmu_page.clone(), spte, level_gfn, level - 1);
            pr_info!(
                "rkvm_tdp_map level={:?}, gfn={:x}, spte={:x} \n",
                level,
                level_gfn,
                spte
            );
            pre_mmu_page = mmu_page;
        }
        level -= 1;
        let tmp  = make_level_gfn(fault.gfn, level);
        level_gfn = match tmp {
          Ok(gfn) => gfn,
          Err(e) => return Err(e),
        };
        let tmp = rkvm_read_spte(pre_mmu_page.clone(), level_gfn, level);
        spte = match tmp {
          Ok(spte) => spte,
          Err(e) => return Err(e),
        }
    } //while
      // handle leaf pte

    if level == fault.goal_level {
        //make pte
        spte = make_spte(fault.pfn);
        //set pte
        rkvm_write_spte(pre_mmu_page, spte, level_gfn, level);
        pr_info!(" ## spte={:x}, level_gfn={:x},level={:?} \n", spte, level_gfn, level);
    }
    Ok(())
}

pub(crate) fn handle_ept_violation(exit_info: &ExitInfo, vcpu: &VcpuWrapper) -> Result<u64> {
    let mut error_code: u64 = 0;
    {
      let mut vcpuinner = vcpu.vcpuinner.lock();
      unsafe {
            (*vcpuinner.run.ptr).exit_reason = exit_info.exit_reason as u32;
      }
    }
    let gpa = vmcs_read64(VmcsField::GUEST_PHYSICAL_ADDRESS);
    pr_info!("handle_ept_violation \n");
    if (exit_info.exit_qualification & EptViolationMask::EPT_VIOLATION_ACC_READ as u64) != 0 {
        error_code = PferrMask::PFERR_USER_MASK as u64;
    }
    if (exit_info.exit_qualification & EptViolationMask::EPT_VIOLATION_ACC_WRITE as u64) != 0 {
        error_code |= PferrMask::PFERR_WRITE_MASK as u64;
    }
    if (exit_info.exit_qualification & EptViolationMask::EPT_VIOLATION_ACC_INSTR as u64) != 0 {
        error_code |= PferrMask::PFERR_FETCH_MASK as u64;
    }
    if exit_info.exit_qualification
        & (EptViolationMask::EPT_VIOLATION_READABLE as u64
            | EptViolationMask::EPT_VIOLATION_WRITABLE as u64
            | EptViolationMask::EPT_VIOLATION_EXECUTABLE as u64)
        != 0
    {
        error_code |= PferrMask::PFERR_PRESENT_MASK as u64;
    }
    if exit_info.exit_qualification & EptViolationMask::EPT_VIOLATION_GVA_TRANSLATED as u64 != 0 {
        error_code |= PferrMask::PFERR_GUEST_FINAL_MASK as u64;
    } else {
        error_code |= PferrMask::PFERR_GUEST_PAGE_MASK as u64;
    }

    let mut fault = RkvmPageFault {
        addr: gpa,
        error_code: error_code,
        present: (error_code & PferrMask::PFERR_PRESENT_MASK as u64) != 0,
        prefetch: false,
        gfn: gpa >> 12,
        pfn: 0,
        hva: 0,
        goal_level: 0,
    };
    pr_info!(" gpa={:x}, error_code={:x}, present={:?} \n", gpa, error_code, fault.present);
    let ret = rkvm_pagefault(vcpu, &mut fault);
    //map
    let ret = rkvm_tdp_map(vcpu, &mut fault);
    match ret {
       Ok(r) => r,
       Err(e) => return Err(e),
    };
    unsafe { bindings::rkvm_invept(2, 0, 0) };
    Ok(1)
}
