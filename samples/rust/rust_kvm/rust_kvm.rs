302 lines (265 sloc)  8.52 KB
   
// SPDX-License-Identifier: GPL-2.0

//! Rust KVM for VMX
//#![feature(asm)]
#[allow(dead_code)]
use kernel::prelude::*;
use kernel::{
    file::File,
    file_operations::{FileOperations, IoctlCommand, IoctlHandler},
    miscdev, pages::Pages, bit, bindings,
    sync::{CondVar, Mutex, Ref, RefBorrow, UniqueRef},
    user_ptr::UserSlicePtrReader,
    Result,
    mm::virt::Area,
    io_buffer::IoBufferReader,
    c_types::c_void,
};

mod exit;
mod guest;
mod mmu;
mod vcpu;
mod vmcs;
mod x86reg;
mod vmstat;
use crate::x86reg::Cr4;
use crate::guest::Guest;
use crate::vcpu::Vcpu;
use crate::vmcs::*;
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

struct KVM;
impl FileOperations for KVM {
    type Wrapper = Ref<RkvmState>;
    type OpenData = Ref<RkvmState>;

    kernel::declare_file_operations!(ioctl, mmap);

    fn open(shared: &Ref<RkvmState>, _file: &File) -> Result<Self::Wrapper> {
        pr_info!("KVM open \n");
        Ok(shared.clone())
    }

    fn mmap(_shared: RefBorrow<'_, RkvmState>, _file: &File, _vma: &mut Area) -> Result {
        pr_info!("KVM mmap \n");
        unsafe {
            bindings::rkvm_mmap(_file.ptr, _vma.vma);
        }
        Ok(())
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
            _dev: miscdev::Registration::new_pinned(fmt!("{name}"), state)?,
        })
    }
}

impl Drop for RustMiscdev {
    fn drop(&mut self) {
        pr_info!("Rust kvm device sample (exit)\n");
    }
}

fn rkvm_set_vmxon(state: &RkvmState) -> Result<u32> {
    // allocate page for vmxon
    let page = Pages::<0>::new();

    let page = match page {
        Ok(page) => page,
        Err(err) => return Err(/*Error::ENOMEM*/ err),
    };

    let vmxinfo = VmxInfo {
        revision_id: state.vmcsconf.revision_id,
        region_size: state.vmcsconf.size as u16,
        write_back: false,
        io_exit_info: false,
        vmx_controls: true,
    };

    let mut kva: u64 = 0;
    unsafe {
        kva = bindings::rkvm_page_address(page.pages);
        let len = core::mem::size_of::<VmxInfo>();
        let p = &vmxinfo;
        pr_info!(
            "Rust kvm:kva={:x}, size={:?},revision={:?} \n",
            kva,
            state.vmcsconf.size,
            vmxinfo.revision_id
        );
        let ptr = core::slice::from_raw_parts((p as *const VmxInfo) as *const u8, len);

        page.write(ptr.as_ptr(), 0, len);
    }

    let vmxe = unsafe {
        bindings::native2_read_cr4() & bit(x86reg::Cr4::CR4_ENABLE_VMX)
    };

    pr_info!("Rust kvm :vmxe {:}\n", vmxe);
    if vmxe > 0 {
        pr_info!("Rust kvm: vmx has been enabled\n");
        return Err(Error::ENOENT);
    }
    unsafe {
        let pa = bindings::rkvm_phy_address(kva);
        pr_info!(" pa = {:x}\n", pa);
        bindings::rkvm_vmxon(pa);
    }
    Ok(0)
}

const IOCTL_KVM_CREATE_VM: u32 = 0x00AE0100;
const IOCTL_KVM_CREATE_VCPU: u32 = 0x00AE4100;
const IOCTL_KVM_VCPU_RUN: u32 = 0x00AE8000;
const IOCTL_KVM_SET_USER_MEMORY_REGION: u32 = 0x40AE4600;

#[repr(C)]
#[allow(dead_code)]
struct RkvmUserspaceMemoryRegion {
    slot: u32,
    flags: u32,
    guest_phys_addr: u64,
    memory_size: u64,    //bytes
    userspace_addr: u64, //start of the userspace allocated memory
}

static mut GUEST: Option<Ref<Mutex<Guest>>> = None;
static mut VCPU: Option<Ref<Mutex<Vcpu>>> = None;

impl IoctlHandler for RkvmState {
    type Target<'a> = &'a RkvmState;

    fn pure(_shared: &RkvmState, file: &File, cmd: u32, _arg: usize) -> Result<i32> {
        match cmd {
            IOCTL_KVM_CREATE_VM => {
                if let Err(error) = rkvm_set_vmxon(_shared) {
                    pr_err!("Rkvm: IOCTL_KVM_CREATE_VM failed\n");
                    return Err(error);
                }
                let guest = Guest::new();

                let guest = match guest {
                    Err(error) => return Err(error),
                    Ok(guest) => guest,
                };
                unsafe {
                    GUEST = Some(guest);
                }
                pr_info!("Rust kvm: IOCTL_KVM_CREATE_VM\n");
                Ok(0)
            }
            IOCTL_KVM_CREATE_VCPU => {
                pr_info!("Rust kvm: IOCTL_KVM_CREATE_VCPU\n");
                let guest = unsafe {
                    match &GUEST {
                        Some(e) => e.clone(),
                        None => return Err(Error::ENOENT),
                    }
                };

                let vcpu0 = Vcpu::new(guest);

                let vcpu0 = match vcpu0 {
                    Err(error) => return Err(error),
                    Ok(vcpu0) => vcpu0,
                };

                vcpu0.lock().init(&_shared.vmcsconf);
                let va = vcpu0.lock().get_run();

                unsafe {
                    //use for mmap
                    (*file.ptr).private_data = va as *mut c_void;
                    VCPU = Some(vcpu0);
                }
                Ok(0)
            }
            IOCTL_KVM_VCPU_RUN => {
                pr_info!("Rust kvm: IOCTL_KVM_VCPU_RUN\n");
                let vcpu = unsafe {
                    match &VCPU {
                        Some(vcpu) => vcpu,
                        None => return Err(Error::ENOENT),
                    }
                };
                let vcpu = vcpu.clone();
                let ret = vcpu.lock().vcpu_run();

                Ok(ret.try_into().unwrap())
            }
            _ => Err(Error::EINVAL),
        }
    }
    fn write(
        _shared: &RkvmState,
        _file: &File,
        cmd: u32,
        reader: &mut UserSlicePtrReader,
    ) -> Result<i32> {
        match cmd {
            IOCTL_KVM_SET_USER_MEMORY_REGION => {
                pr_info!("Rust kvm: IOCTL_KVM_SET_USER_MEMORY_REGION\n");
                let guest = unsafe {
                    match &GUEST {
                        Some(e) => e.clone(),
                        None => return Err(Error::ENOENT),
                    }
                };
                let mut uaddr_ = RkvmUserspaceMemoryRegion {
                    slot: 0,
                    flags: 0,
                    guest_phys_addr: 0,
                    memory_size: 0,
                    userspace_addr: 0,
                };
                let len = reader.len();
                unsafe {
                    let mut ptr = core::slice::from_raw_parts_mut(
                        (&mut uaddr_ as *mut RkvmUserspaceMemoryRegion) as *mut u8,
                        len,
                    );
                    reader.read_raw(ptr.as_mut_ptr(), len)?;
                }
                let ret = guest.lock().add_memory_region(
                    uaddr_.userspace_addr,
                    uaddr_.memory_size >> 12,
                    uaddr_.guest_phys_addr,
                )?;
               Ok(ret.try_into().unwrap())
            }
            _ => Err(Error::EINVAL),
        }
    }
}