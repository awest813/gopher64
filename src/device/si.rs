use crate::device;
use crate::ui;
use rand::Rng;

const SI_DRAM_ADDR_REG: usize = 0;
const SI_PIF_ADDR_RD64B_REG: usize = 1;
//const SI_R2_REG: usize = 2;
//const SI_R3_REG: usize = 3;
const SI_PIF_ADDR_WR64B_REG: usize = 4;
//const SI_R5_REG: usize = 5;
pub const SI_STATUS_REG: usize = 6;
pub const SI_REGS_COUNT: usize = 7;

pub const SI_STATUS_DMA_BUSY: u32 = 1 << 0;
pub const SI_STATUS_IO_BUSY: u32 = 1 << 1;
//const SI_STATUS_DMA_ERROR: u32 = 1 << 3;
const SI_STATUS_INTERRUPT: u32 = 1 << 12;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DmaDir {
    None,
    Write,
    Read,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Si {
    pub regs: [u32; SI_REGS_COUNT],
    pub dma_dir: DmaDir,
}

fn read_u32_be_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(
        bytes
            .get(offset..offset + 4)
            .and_then(|slice| slice.try_into().ok())
            .unwrap_or([0; 4]),
    )
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    let reg = ((address & 0xFFFF) >> 2) as usize;
    if reg >= SI_REGS_COUNT {
        eprintln!("Unknown SI register read {reg} at {address:#x}");
        return 0;
    }
    device.si.regs[reg]
}

fn randomize_interrupt_time(rng: &mut rand::rngs::Xoshiro256PlusPlus) -> u64 {
    rng.next_u64() % 0x100
}

fn dma_read(device: &mut device::Device) {
    device.si.dma_dir = DmaDir::Read;

    let duration = device::pif::update_pif_ram(device);

    device.si.regs[SI_STATUS_REG] |= SI_STATUS_DMA_BUSY;

    let length = duration + randomize_interrupt_time(&mut device.rng);

    device::events::create_event(device, device::events::EVENT_TYPE_SI, length)
}

fn dma_write(device: &mut device::Device) {
    device.si.dma_dir = DmaDir::Write;

    copy_pif_rdram(device);

    device.si.regs[SI_STATUS_REG] |= SI_STATUS_DMA_BUSY;

    let length = 6000 + randomize_interrupt_time(&mut device.rng); //based on https://github.com/rasky/n64-systembench

    device::events::create_event(device, device::events::EVENT_TYPE_SI, length)
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = ((address & 0xFFFF) >> 2) as usize;
    if reg >= SI_REGS_COUNT {
        eprintln!("Unknown SI register write {reg} at {address:#x}");
        return;
    }
    match reg {
        SI_STATUS_REG => {
            device.si.regs[reg] &= !SI_STATUS_INTERRUPT;
            device::mi::clear_rcp_interrupt(device, device::mi::MI_INTR_SI)
        }
        SI_PIF_ADDR_RD64B_REG => dma_read(device),
        SI_PIF_ADDR_WR64B_REG => dma_write(device),
        _ => device::memory::masked_write_32(&mut device.si.regs[reg], value, mask),
    }
}

//rdram is in native endian format, and pif memory is in big endian format
fn copy_pif_rdram(device: &mut device::Device) {
    let dram_addr = device.si.regs[SI_DRAM_ADDR_REG] as usize & device::rdram::RDRAM_MASK;
    if device.si.dma_dir == DmaDir::Write {
        let mut i = 0;
        while i < device::pif::PIF_RAM_SIZE {
            let data = u32::from_ne_bytes(
                device
                    .rdram
                    .mem
                    .get(dram_addr + i..dram_addr + i + 4)
                    .unwrap_or(&[0; 4])
                    .try_into()
                    .unwrap_or_default(),
            );
            device.pif.ram[i..i + 4].copy_from_slice(&data.to_be_bytes());
            i += 4;
        }
    } else if device.si.dma_dir == DmaDir::Read {
        // check RDP before SI writes to RDRAM
        ui::video::check_framebuffers(dram_addr as u32, device::pif::PIF_RAM_SIZE as u32);
        let mut i = 0;
        while i < device::pif::PIF_RAM_SIZE {
            let data = read_u32_be_at(&device.pif.ram, i);
            device
                .rdram
                .mem
                .get_mut(dram_addr + i..dram_addr + i + 4)
                .unwrap_or(&mut [0; 4])
                .copy_from_slice(&data.to_ne_bytes());
            i += 4;
        }
    } else {
        eprintln!("SI DMA with unknown direction {:?}", device.si.dma_dir);
    }
}

pub fn dma_event(device: &mut device::Device) {
    if device.si.dma_dir == DmaDir::Write {
        device::pif::process_ram(device);
    } else if device.si.dma_dir == DmaDir::Read {
        device::si::copy_pif_rdram(device);
    } else {
        eprintln!("SI DMA event with unknown direction {:?}", device.si.dma_dir);
    }
    device.si.dma_dir = DmaDir::None;
    device.si.regs[SI_STATUS_REG] &= !(SI_STATUS_DMA_BUSY | SI_STATUS_IO_BUSY);
    device.si.regs[SI_STATUS_REG] |= SI_STATUS_INTERRUPT;

    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_SI)
}

#[cfg(test)]
mod tests {
    #[test]
    fn unknown_si_register_read_returns_zero() {
        let mut device = *crate::device::Device::new(false);
        let value = super::read_regs(
            &mut device,
            0x0480_0100,
            crate::device::memory::AccessSize::Word,
        );
        assert_eq!(value, 0);
    }

    #[test]
    fn unknown_si_register_write_is_ignored() {
        let mut device = *crate::device::Device::new(false);
        device.si.regs[0] = 0x1234_5678;
        super::write_regs(&mut device, 0x0480_0100, 0xDEAD_BEEF, 0xFFFF_FFFF);
        assert_eq!(device.si.regs[0], 0x1234_5678);
    }
}
