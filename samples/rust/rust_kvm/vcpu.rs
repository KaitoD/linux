#[allow(dead_code)]
use kernel::prelude::*;
use kernel::{
    sync::{CondVar, Mutex, Ref, RefBorrow, UniqueRef, Box},
};
mod vmcs;
use crate::vmcs::*;

mod vmstat;
use crate::vmstat::*;

#[allow(dead_code)]
pub(crate) struct Vcpu {
    pub(crate) guest: Option<Ref<Mutex<Guest>>>,
    pub(crate) vmx_state: Option<Box<VmxState>>,
    pub(crate) run: u64, // kvm_run
    pub(crate) vcpu_id: Option<u32>,
    pub(crate) launched: bool,
}

impl Vcpu {
    fn new() -> Result<Self> {
        Vcpu {
            guest: None,
            vmx_state: None,
            run: 0,
            vcpu_id: None,
            launched: false,
        }
    }

    fn set_id(id: u32) -> u32 {
        match self.vcpu_id {
            Some(i) => {
                pr_info!("Rust kvm: changing vcpu id is not allowed.\n");
                -1
            }
            None => {
                self.vcpu_id = id;
                0
            }
        };
    }
}