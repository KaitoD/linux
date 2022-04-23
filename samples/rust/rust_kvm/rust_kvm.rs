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
};

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

struct KVM {
    mm: Option<Arc<MemoryMap>>, // TODO: Allocate userspace tied to this VM
    kvm_memslots: Option<Arc<Memslots>>, // TODO: implement Memory Slot
    kvm_vcpus: Vec<Mutex<Vcpus>>, // TODO: implement Vcpu
    online_vcpus: Option<Mutex<u32>>,
    last_boosted_vcpus: Option<u32>,
    user_counts: Option<u32>
    vm_list: Vec<Mutex<RkvmState>>,
    kvm_vm_stat: Option<Mutex<VmxState>>,
};

impl KVM {
    fn new() -> Self {
        mm: None,
        kvm_memslots: None,
        kvm_vcpus: Vec::new(),
        online_vcpus: None,
        last_boosted_vcpus: None,
        user_counts: None,
        vm_list: Vec::new(),
        kvm_vm_stat: None,
    }
    fn create_vm(vm_type: u32, ram_size : u32) -> Self {
        // TODO: alloc a new GuestPhysMemorySet
        // if map fail, return -1
        // set memslot 0, gpm addr 0, userspace for ram
    }

    fn create_vcpu(vcpu_id : u32) -> Self {
        // TODO: new a vcpu
        // vcpu mmap
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
