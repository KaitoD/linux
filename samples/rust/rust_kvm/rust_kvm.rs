// SPDX-License-Identifier: GPL-2.0

//! Rust KVM for VMX
//#![feature(asm)]
#[allow(dead_code)]
use kernel::prelude::*;
use kernel::{
    file::File,
    file_operations::{FileOperations, IoctlCommand, IoctlHandler},
    miscdev,
    sync::{CondVar, Mutex, Ref, RefBorrow, UniqueRef},
    task::mm_struct,
};
use memory::*;

mod vmcs;
use crate::vmcs::*;

mod vcpu;
use crate::vcpu::*;
module! {
    type: RustMiscdev,
    name: b"rust_kvm",
    author: b"Peng Hao",
    description: b"Rust KVM VMX",
    license: b"GPL v2",
}

struct SharedStateInner {
    token_count: usize,
}

struct RkvmState {
    vmcsconf: VmcsConfig,
    state_changed: CondVar,
    inner: Mutex<SharedStateInner>,
}

// used for vmxon
struct VmxInfo {
    revision_id: u32,
    region_size: u16,
    write_back: bool,
    io_exit_info: bool,
    vmx_controls: bool,
}

impl RkvmState {
    fn try_new() -> Result<Ref<Self>> {
        pr_info!("RkvmState try_new \n");
        let mut vmcsconf = VmcsConfig::new()?;
        vmcsconf.setup_config();
        let mut state = Pin::from(UniqueRef::try_new(Self {
            // SAFETY: `condvar_init!` is called below
            vmcsconf: vmcsconf,
            state_changed: unsafe { CondVar::new() },
            // SAFETY: `mutex_init!` is called below.
            inner: unsafe { Mutex::new(SharedStateInner { token_count: 0 }) },
        })?);

        // SAFETY: `state_changed` is pinned when `state` is.
        let pinned = unsafe { state.as_mut().map_unchecked_mut(|s| &mut s.state_changed) };
        kernel::condvar_init!(pinned, "RkvmState::state_changed");

        // SAFETY: `inner` is pinned when `state` is.
        let pinned = unsafe { state.as_mut().map_unchecked_mut(|s| &mut s.inner) };
        kernel::mutex_init!(pinned, "RkvmState::inner");

        Ok(state.into())
    }
}

struct kvm_vm_state{
    state : bool ,
    mmu_pte_write :Option<u32> ,
    mmu_pte_updated : Option<u32> ,
}

impl kvm_vm_state {
    fn new() -> kvm_vm_state {
        kvm_vm_state{
            state : false,
            mmu_pte_write : None,
            mmu_pte_updated : None,
        }
    }
    fn kvm_change_state(&mut self , new_stat : bool ) -> bool{
        self.state = new_state;
        true
    }
}

struct KVM {
    mm: Option<Arc<mm_struct>>,
    memslots: Vec<Option<DefaultGuestPhysMemorySet>>,
    vcpus: Vec<Mutex<Vcpu>>,
    user_counts: Option<u32>
    state: Option<Mutex<VmxState>>,
};

impl KVM {
    fn new() -> Self {
        mm: None,
        memslots: None,
        vcpus: Vec::new(),
        user_counts: None,
        state: None, 
    }
    fn create_vm(&mut self,kvm_type : u64 , mem_size : u8) -> Self {
        // init user_count
        self.user_count = match self.user_count {
            Some(i) => Some(i+1),
            None => Some(1),
        };

        // init state
        self.state = true;

        // init mm
        self.mm = current.mm;   //存疑，封装current

        // init memslots
        for i in 0..mem_size{
            self.memslots.push(None);
        }
        // TODO: arch func
        self
    }

    fn create_vcpu(vcpu_id : u32) -> Self {
        let mut new_vcpu = Vcpu::new();
        new_vcpu.set_id(vcpu_id);
        
    }

    // TODO: load bin

}

impl FileOperations for KVM {
    type Wrapper = Ref<RkvmState>;
    type OpenData = Ref<RkvmState>;

    kernel::declare_file_operations!(ioctl);

    fn open(shared: &Ref<RkvmState>, _file: &File) -> Result<Self::Wrapper> {
        pr_info!("KVM open \n");
        Ok(shared.clone())
    }

    fn ioctl(shared: RefBorrow<'_, RkvmState>, file: &File, cmd: &mut IoctlCommand) -> Result<i32> {
        cmd.dispatch::<RkvmState>(&shared, file)
    }
}

struct RustMiscdev {
    _dev: Pin<Box<miscdev::Registration<KVM>>>,
}

impl KernelModule for RustMiscdev {
    fn init(name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust kvm device init\n");

        let state = RkvmState::try_new()?;
        /* vmxon percpu*/

        Ok(RustMiscdev {
            _dev: miscdev::Registration::new_pinned(name, state)?,
        })
    }
}

impl Drop for RustMiscdev {
    fn drop(&mut self) {
        pr_info!("Rust kvm device sample (exit)\n");
    }
}

const IOCTL_KVM_CREATE_VM: u32 = 0x00AE0100;
const IOCTL_KVM_CREATE_VCPU: u32 = 0x00AE4100;

impl IoctlHandler for RkvmState {
    type Target<'a> = &'a RkvmState;

    fn pure(_shared: &RkvmState, _: &File, cmd: u32, _arg: usize) -> Result<i32> {
        match cmd {
            IOCTL_KVM_CREATE_VM => {
                pr_info!("Rust kvm: IOCTL_KVM_CREATE_VM\n");
                Ok(0)
            }
            IOCTL_KVM_CREATE_VCPU => {
                pr_info!("Rust kvm: IOCTL_KVM_CREATE_VCPU\n");
                Ok(0)
            }
            _ => Err(Error::EINVAL),
        }
    }
}
