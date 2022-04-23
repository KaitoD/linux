mod vmcs;
use crate::vmcs::*;

#[allow(dead_code)]
pub(crate) struct Vcpu {
    pub(crate) guest: Ref<Mutex<Guest>>,
    pub(crate) vmx_state: Box<VmxState>,
    pub(crate) run: u64,
    pub(crate) vcpu_id: u32,
    pub(crate) launched: bool,
}