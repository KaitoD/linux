pub type GuestPhysAddr = usize;
pub type HostPhysAddr = usize;
pub type HostVirtAddr = usize;

pub type KvmResult<T = ()> = Result<T, KvmError>;
#[derive(Debug, PartialEq)]
pub enum KvmError {
    Internal,
    NotSupported,
    NoMemory,
    InvalidParam,
    OutOfRange,
    BadState,
    NotFound,
}

pub struct KvmMemorySlot {
    struct hlist_node id_node[2];
	struct interval_tree_node hva_node[2];
	struct kvm_arch_memory_slot arch;
	userspace_addr : u32;
	flags : u32;
	id : u16;
	as_id : u16;
}

pub trait IntoKvmPageTableFlags {
    fn is_read(&self) -> bool;
    fn is_write(&self) -> bool;
    fn is_execute(&self) -> bool;
}

pub trait KvmPageTable {
    /// Map a guest physical frame starts from `gpaddr` to the host physical
    /// frame starts from of `hpaddr` with `flags`.
    fn map(
        &mut self,
        gpaddr: GuestPhysAddr,
        hpaddr: HostPhysAddr,
        flags: impl IntoKvmPageTableFlags,
    ) -> KvmResult;

    /// Unmap the guest physical frame `hpaddr`.
    fn unmap(&mut self, gpaddr: GuestPhysAddr) -> KvmResult;

    /// Change the `flags` of the guest physical frame `gpaddr`.
    fn protect(&mut self, gpaddr: GuestPhysAddr, flags: impl IntoKvmPageTableFlags) -> KvmResult;

    /// Query the host physical address which the guest physical frame of
    /// `gpaddr` maps to.
    fn query(&mut self, gpaddr: GuestPhysAddr) -> KvmResult<HostPhysAddr>;

    /// Page table base address.
    fn table_phys(&self) -> HostPhysAddr;
}

pub trait GuestPhysMemorySetTrait {
    /// Physical address space size.
    fn size(&self) -> u64;

    /// Read from guest address space.
    fn read_memory(&self, gpaddr: GuestPhysAddr, buf: &mut [u8]) -> KvmResult<usize>;

    /// Write to guest address space.
    fn write_memory(&self, gpaddr: GuestPhysAddr, buf: &[u8]) -> KvmResult<usize>;
}

#[derive(Debug, Clone, Copy)]
pub struct GuestMemoryAttr {
    read: bool,
    write: bool,
    execute: bool,
}

impl GuestMemoryAttr {
    fn empty() -> Self {
        Self {
            read: false,
            write: false,
            execute: false,
        }
    }
    fn is_read(&self) -> bool {
        self.read
    }
    fn is_write(&self) -> bool {
        self.write
    }
    fn is_execute(&self) -> bool {
        self.execute
    }
}

impl Default for GuestMemoryAttr {
    fn default() -> Self {
        Self {
            read: true,
            write: true,
            execute: true,
        }
    }
    fn is_read(&self) -> bool {
        self.read
    }
    fn is_write(&self) -> bool {
        self.write
    }
    fn is_execute(&self) -> bool {
        self.execute
    }
}

#[derive(Debug)]
pub struct GuestPhysMemoryRegion {
    start_paddr: GuestPhysAddr,
    end_paddr: GuestPhysAddr,
    attr: GuestMemoryAttr,
}

impl GuestPhysMemoryRegion {
    /// Test whether a guest physical address is in the memory region
    fn contains(&self, guest_paddr: GuestPhysAddr) -> bool {
        self.start_paddr <= guest_paddr && guest_paddr < self.end_paddr
    }

    /// Test whether this region is (page) overlap with region [`start_paddr`, `end_paddr`)
    fn is_overlap_with(&self, start_paddr: GuestPhysAddr, end_paddr: GuestPhysAddr) -> bool {
        let p0 = self.start_paddr;
        let p1 = self.end_paddr;
        let p2 = start_paddr;
        let p3 = end_paddr;
        !(p1 <= p2 || p0 >= p3)
    }
}

#[derive(Debug)]
pub struct DefaultGuestPhysMemorySet {
    pub regions: Mutex<Vec<GuestPhysMemoryRegion>>,
}

impl DefaultGuestPhysMemorySet {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            regions: Mutex::new(Vec::new()),
        })
    }

    fn find_region<F, T>(&self, gpaddr: GuestPhysAddr, op: F) -> KvmResult<T>
    where
        F: FnOnce(&GuestPhysMemoryRegion) -> KvmResult<T>,
    {
        if let Some(region) = self
            .regions
            .lock()
            .iter()
            .find(|region| region.contains(gpaddr))
        {
            op(region)
        } else {
            Err(KvmError::NotFound)
        }
    }

    /// Test if [`start_paddr`, `end_paddr`) is a free region.
    fn test_free_region(&self, start_paddr: GuestPhysAddr, end_paddr: GuestPhysAddr) -> bool {
        self.regions
            .lock()
            .iter()
            .any(|region| region.is_overlap_with(start_paddr, end_paddr))
    }
}

impl GuestPhysMemorySetTrait for DefaultGuestPhysMemorySet {
    fn size(&self) -> u64 {
        1 << 32
    }

    fn read_memory(&self, gpaddr: GuestPhysAddr, buf: &mut [u8]) -> KvmResult<usize> {
        let size = buf.len();
        let hvaddr = self.query_range(gpaddr, size)?;
        unsafe { buf.copy_from_slice(core::slice::from_raw_parts(hvaddr as *const u8, size)) }
        Ok(size)
    }

    fn write_memory(&self, gpaddr: GuestPhysAddr, buf: &[u8]) -> KvmResult<usize> {
        let size = buf.len();
        let hvaddr = self.query_range(gpaddr, size)?;
        unsafe { core::slice::from_raw_parts_mut(hvaddr as *mut u8, size).copy_from_slice(buf) }
        Ok(size)
    }
    
}

impl Drop for DefaultGuestPhysMemorySet {
    fn drop(&mut self) {
        self.clear();
    }
}