// SPDX-License-Identifier: GPL-2.0
use kernel::{
    bindings,
    linked_list::{GetLinks, Links, List},
    pages::Pages,
    prelude::*,
    sync::{Ref, UniqueRef},
    Result,
};

use crate::vmcs::*;
//use alloc::vec::Vec;
#[repr(u64)]
#[derive(Debug)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum VmxEptpFlag {
    VMX_EPTP_PWL_MASK = 0x38,
    VMX_EPTP_PWL_4 = 0x18,
    VMX_EPTP_PWL_5 = 0x20,
    VMX_EPTP_AD_ENABLE_BIT = (1 << 6),
    VMX_EPTP_MT_MASK = 0x7,
    VMX_EPTP_MT_WB = 0x6,
    VMX_EPTP_MT_UC = 0x0,
}

#[repr(u64)]
#[derive(Debug)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum VmxEptFlag {
    VMX_EPT_READABLE_MASK = 0x1,
    VMX_EPT_WRITABLE_MASK = 0x2,
    VMX_EPT_EXECUTABLE_MASK = 0x4,
    VMX_EPT_IPAT_BIT = (1 << 6),
    VMX_EPT_ACCESS_BIT = (1 << 8),
    VMX_EPT_DIRTY_BIT = (1 << 9),
}

pub(crate) struct RkvmMmu {
    pub(crate) root_hpa: u64,
    pub(crate) root_mmu_page: Ref<RkvmMmuPage>,
    mmu_pages_list: List<Ref<RkvmMmuPage>>,
}

impl RkvmMmu {
    pub(crate) fn new() -> Result<UniqueRef<Self>> {
        let root = RkvmMmuPage::new(true, 4, None); //root level = 4
        let root = match root {
            Ok(root) => root,
            Err(err) => return Err(err),
        };
        let hpa = match root.spt {
            Some(hpa) => hpa,
            None => return Err(Error::ENOMEM),
        };
        let mut hpa = unsafe { bindings::rkvm_phy_address(hpa) };

        let mut mmu = UniqueRef::try_new(Self {
            root_hpa: hpa, //physical addr
            root_mmu_page: root.clone(),
            mmu_pages_list: List::new(),
        })?;

        mmu.mmu_pages_list.push_back(root);
        Ok(mmu)
        //let guest_cr3 =
    }
    pub(crate) fn alloc_mmu_page(&mut self, level: u64, gfn: u64) -> Result<Ref<RkvmMmuPage>> {
        let mmu_page = RkvmMmuPage::new(false, level, Some(gfn));
        let mmu_page = match mmu_page {
            Ok(page) => page,
            Err(err) => return Err(err),
        };
        let ret = mmu_page.clone();
        self.mmu_pages_list.push_back(mmu_page);
        Ok(ret)
    }

    pub(crate) fn init_mmu_root(&mut self) -> Result {
        ///TODO: pgd setting
        let mut eptp: u64 = VmxEptpFlag::VMX_EPTP_MT_WB as u64 | VmxEptpFlag::VMX_EPTP_PWL_4 as u64;
        eptp |= self.root_hpa;
        vmcs_write64(VmcsField::EPT_POINTER, eptp);
        Ok(())
    }
    pub(crate) fn is_pte_present(&self, pte: u64) -> bool {
        (pte & (1u64 << 11)) != 0u64
    }
}

#[allow(dead_code)]
pub(crate) struct RkvmMmuPage {
    pub(crate) gfn: Option<u64>,
    pub(crate) pages: Pages<0>,
    pub(crate) root: bool,
    pub(crate) spt: Option<u64>, //mmu page's vaddr
    pub(crate) level: u64,
    pub(crate) page_links: Links<RkvmMmuPage>,
}

impl RkvmMmuPage {
    pub(crate) fn new(isroot: bool, level: u64, gfn: Option<u64>) -> Result<Ref<Self>> {
        let page = Pages::<0>::new();
        let page = match page {
            Ok(page) => page,
            Err(err) => return Err(err),
        };
        let spt = unsafe { Some(bindings::rkvm_page_address(page.pages)) };

        let mmu_page = Ref::try_new(RkvmMmuPage {
            gfn: gfn,
            pages: page,
            root: isroot,
            spt: spt,
            level: level,
            page_links: Links::new(),
        })?;

        Ok(mmu_page)
    }
} //

impl GetLinks for RkvmMmuPage {
    type EntryType = RkvmMmuPage;
    fn get_links(data: &RkvmMmuPage) -> &Links<RkvmMmuPage> {
        &data.page_links
    }
}