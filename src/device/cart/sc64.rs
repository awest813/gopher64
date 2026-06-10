use crate::device;
use crate::ui;

pub const SDCARD_SIZE: usize = 0x4000000;

const SC64_SCR_REG: usize = 0;
const SC64_DATA0_REG: usize = 1;
const SC64_DATA1_REG: usize = 2;
const SC64_IDENTIFIER_REG: usize = 3;
const SC64_KEY_REG: usize = 4;
//const SC64_IRQ_REG: usize = 5;
//const SC64_AUX_REG: usize = 6;
pub const SC64_REGS_COUNT: usize = 7;

pub const SC64_BOOTLOADER_SWITCH: u32 = 0;
pub const SC64_ROM_WRITE_ENABLE: u32 = 1;
pub const SC64_SAVE_TYPE: u32 = 6;
pub const SC64_CFG_COUNT: usize = 15;

const SC64_BUFFER_MASK: usize = 0x1FFF;
const SC64_EEPROM_MASK: usize = 0xFFF;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Sc64 {
    pub buffer: Vec<u8>,
    pub regs: [u32; SC64_REGS_COUNT],
    pub regs_locked: bool,
    pub cfg: [u32; SC64_CFG_COUNT],
    pub sector: u32,
    pub writeback_sector: Vec<u32>,
    pub usb_buffer: Vec<u8>,
}

fn format_sdcard(device: &mut device::Device) {
    if device.ui.storage.saves.sdcard.data.is_empty() {
        device.ui.storage.saves.sdcard.data.resize(SDCARD_SIZE, 0);
        let buf = std::io::Cursor::new(&mut device.ui.storage.saves.sdcard.data);
        if let Err(err) = fatfs::format_volume(buf, fatfs::FormatVolumeOptions::new()) {
            eprintln!("Failed to format SC64 SD card: {err}");
        }
    }
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    if device.cart.sc64.regs_locked {
        return 0;
    }
    let reg = ((address & 0xFFFF) >> 2) as usize;
    if reg >= SC64_REGS_COUNT {
        eprintln!("unknown SC64 read reg {reg} at {address:#x}");
        return 0;
    }
    match reg {
        SC64_SCR_REG | SC64_DATA0_REG | SC64_DATA1_REG => device.cart.sc64.regs[reg],
        SC64_IDENTIFIER_REG => 0x53437632,
        _ => {
            eprintln!("unknown SC64 read reg {reg} at {address:#x}");
            0
        }
    }
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = ((address & 0xFFFF) >> 2) as usize;
    if reg >= SC64_REGS_COUNT {
        eprintln!("unknown SC64 write reg {reg} at {address:#x}");
        return;
    }
    match reg {
        SC64_KEY_REG => {
            device::memory::masked_write_32(&mut device.cart.sc64.regs[reg], value, mask);
            if device.cart.sc64.regs[SC64_KEY_REG] == 0x4F434B5F {
                device.cart.sc64.regs_locked = false;
            } else if device.cart.sc64.regs[SC64_KEY_REG] == 0xFFFFFFFF {
                device.cart.sc64.regs_locked = true;
            }
        }
        SC64_DATA0_REG | SC64_DATA1_REG => {
            if !device.cart.sc64.regs_locked {
                device::memory::masked_write_32(&mut device.cart.sc64.regs[reg], value, mask);
            }
        }
        SC64_SCR_REG => {
            if !device.cart.sc64.regs_locked {
                let Some(cmd) = char::from_u32(value & mask) else {
                    eprintln!("Invalid SC64 command value {value:#x} at {address:#x}");
                    return;
                };
                match cmd {
                    'V' => {
                        // get version
                        device.cart.sc64.regs[SC64_DATA0_REG] = (2 << 16) | 20;
                        device.cart.sc64.regs[SC64_DATA1_REG] = 2;
                    }
                    'c' => {
                        // get config
                        let cfg_index = device.cart.sc64.regs[SC64_DATA0_REG] as usize;
                        device.cart.sc64.regs[SC64_DATA1_REG] = device
                            .cart
                            .sc64
                            .cfg
                            .get(cfg_index)
                            .copied()
                            .unwrap_or(0);
                    }
                    'C' => {
                        // set config
                        if device.cart.sc64.regs[SC64_DATA0_REG] == SC64_SAVE_TYPE {
                            // if save type is being written, we are probably booting a game using the flash cart menu
                            // we shouldn't write saves to disk in this case (they are written to the SD card)
                            device.ui.storage.saves.write_to_disk = false;
                            device.ui.storage.save_type =
                                match device.cart.sc64.regs[SC64_DATA1_REG] {
                                    0 => {
                                        vec![]
                                    }
                                    1 => {
                                        vec![ui::storage::SaveTypes::Eeprom4k]
                                    }
                                    2 => {
                                        vec![ui::storage::SaveTypes::Eeprom16k]
                                    }
                                    3 => {
                                        vec![ui::storage::SaveTypes::Sram]
                                    }
                                    4 => {
                                        vec![ui::storage::SaveTypes::Flash]
                                    }
                                    _ => {
                                        eprintln!(
                                            "unknown SC64 save type: {}",
                                            device.cart.sc64.regs[SC64_DATA1_REG]
                                        );
                                        vec![]
                                    }
                                }
                        }
                        std::mem::swap(
                            &mut device.cart.sc64.cfg
                                [device.cart.sc64.regs[SC64_DATA0_REG] as usize],
                            &mut device.cart.sc64.regs[SC64_DATA1_REG],
                        );
                    }
                    'i' => {
                        // sd card operation
                        match device.cart.sc64.regs[SC64_DATA1_REG] {
                            0 => { //Init SD card
                            }
                            1 => { //Deinit SD card
                            }
                            _ => {
                                eprintln!(
                                    "unknown SC64 SD card operation: {}",
                                    device.cart.sc64.regs[SC64_DATA1_REG]
                                );
                            }
                        }
                    }
                    'I' => {
                        // set sd sector
                        device.cart.sc64.sector = device.cart.sc64.regs[SC64_DATA0_REG];
                    }
                    's' => {
                        format_sdcard(device);
                        // read sd card
                        let address = device.cart.sc64.regs[SC64_DATA0_REG] as u64 & 0x1FFFFFFF;
                        let offset = (device.cart.sc64.sector * 512) as usize;
                        let length = (device.cart.sc64.regs[SC64_DATA1_REG] * 512) as usize;
                        let mut i = 0;

                        while i < length {
                            let data = u32::from_be_bytes(
                                device
                                    .ui
                                    .storage
                                    .saves
                                    .sdcard
                                    .data
                                    .get((offset + i)..(offset + i + 4))
                                    .unwrap_or(&[0; 4])
                                    .try_into()
                                    .unwrap_or_default(),
                            );

                            device::memory::data_write(
                                device,
                                address + i as u64,
                                data,
                                0xFFFFFFFF,
                                false,
                            );
                            i += 4;
                        }
                    }
                    'S' => {
                        format_sdcard(device);
                        // write sd card
                        let address = device.cart.sc64.regs[SC64_DATA0_REG] as u64 & 0x1FFFFFFF;
                        let offset = (device.cart.sc64.sector * 512) as usize;
                        let length = (device.cart.sc64.regs[SC64_DATA1_REG] * 512) as usize;
                        let mut i = 0;

                        while i < length {
                            let data = device::memory::data_read(
                                device,
                                address + i as u64,
                                device::memory::AccessSize::Word,
                                false,
                            )
                            .to_be_bytes();
                            device
                                .ui
                                .storage
                                .saves
                                .sdcard
                                .data
                                .get_mut((offset + i)..(offset + i + 4))
                                .unwrap_or(&mut [0; 4])
                                .copy_from_slice(&data);
                            i += 4;
                        }
                        ui::storage::schedule_save(device, ui::storage::SaveTypes::Sdcard);
                    }
                    'U' => {
                        device.cart.sc64.regs[SC64_DATA0_REG] = 0;
                    }
                    'u' => {
                        // used to notify the game that there is data to read
                        if let Some(cart_rx) = device.ui.usb.cart_rx.as_mut() {
                            match cart_rx.try_recv() {
                                Ok(data) => {
                                    device.cart.sc64.regs[SC64_DATA0_REG] = data.data_type & 0xFF; // read_status/type
                                    device.cart.sc64.regs[SC64_DATA1_REG] =
                                        data.data_size & 0xFFFFFF; // length
                                    device.cart.sc64.usb_buffer = data.data; // store the data to be read
                                }
                                Err(err) => {
                                    match err {
                                        tokio::sync::broadcast::error::TryRecvError::Lagged(_) => {
                                            eprintln!("SC64 cart_rx lagged: {err}");
                                        }
                                        _ => {
                                            device.cart.sc64.regs[SC64_DATA0_REG] = 0; // read_status/type
                                            device.cart.sc64.regs[SC64_DATA1_REG] = 0; // length
                                        }
                                    }
                                }
                            }
                        } else {
                            device.cart.sc64.regs[SC64_DATA0_REG] = 0; // read_status/type
                            device.cart.sc64.regs[SC64_DATA1_REG] = 0; // length
                        }
                    }
                    'M' => {
                        // Send data from from flashcart to USB
                        let address = device.cart.sc64.regs[SC64_DATA0_REG] as u64 & 0x1FFFFFFF;
                        let length = device.cart.sc64.regs[SC64_DATA1_REG] as usize & 0xFFFFFF;

                        if let Some(usb_tx) = device.ui.usb.usb_tx.as_mut() {
                            let mut usb_buffer = vec![0; length];

                            let mut i = 0;

                            if address < device.rdram.size as u64 {
                                while i < length {
                                    *usb_buffer.get_mut(i).unwrap_or(&mut 0) = *device
                                        .rdram
                                        .mem
                                        .get((address as usize + i) ^ device.byte_swap)
                                        .unwrap_or(&0);
                                    i += 1;
                                }
                            } else if address >= device::memory::MM_CART_ROM as u64
                                && address < device::memory::MM_PIF_MEM as u64
                            {
                                while i < length {
                                    *usb_buffer.get_mut(i).unwrap_or(&mut 0) = *device
                                        .ui
                                        .storage
                                        .saves
                                        .romsave
                                        .data
                                        .get(
                                            &(((address as usize + i)
                                                & device::cart::rom::CART_MASK)
                                                as u32),
                                        )
                                        .unwrap_or(
                                            device
                                                .cart
                                                .rom
                                                .get(
                                                    (address as usize + i)
                                                        & device::cart::rom::CART_MASK,
                                                )
                                                .unwrap_or(&0),
                                        );
                                    i += 1;
                                }
                            } else {
                                eprintln!("Unknown address {address:#x} for SC64 M command");
                            }

                            ui::usb::send_to_usb(
                                usb_tx,
                                ui::usb::UsbData {
                                    data: usb_buffer,
                                    data_type: device.cart.sc64.regs[SC64_DATA1_REG] >> 24,
                                    data_size: device.cart.sc64.regs[SC64_DATA1_REG] & 0xFFFFFF,
                                },
                            );
                        }
                    }
                    'm' => {
                        // Receive data from USB to flashcart
                        let address = device.cart.sc64.regs[SC64_DATA0_REG] as u64 & 0x1FFFFFFF;
                        let length = device.cart.sc64.regs[SC64_DATA1_REG] as usize;

                        let mut i = 0;

                        if address < device.rdram.size as u64 {
                            while i < length {
                                *device
                                    .rdram
                                    .mem
                                    .get_mut((address as usize + i) ^ device.byte_swap)
                                    .unwrap_or(&mut 0) =
                                    *device.cart.sc64.usb_buffer.get(i).unwrap_or(&0);
                                i += 1;
                            }
                        } else if address >= device::memory::MM_CART_ROM as u64
                            && address < device::memory::MM_PIF_MEM as u64
                        {
                            while i < length {
                                device.ui.storage.saves.romsave.data.insert(
                                    ((address as usize + i) & device::cart::rom::CART_MASK) as u32,
                                    *device.cart.sc64.usb_buffer.get(i).unwrap_or(&0),
                                );
                                i += 1;
                            }
                        } else {
                            eprintln!("Unknown address {address:#x} for SC64 m command");
                        }
                    }
                    'w' => {
                        // SD card writeback pending
                        device.cart.sc64.regs[SC64_DATA0_REG] = 0;
                    }
                    'W' => {
                        let writeback_sectors_address =
                            device.cart.sc64.regs[SC64_DATA0_REG] as u64;
                        for i in 0..256 {
                            device.cart.sc64.writeback_sector[i] = device::memory::data_read(
                                device,
                                writeback_sectors_address + (i * 4) as u64,
                                device::memory::AccessSize::Word,
                                false,
                            );
                        }
                    }
                    _ => {
                        eprintln!(
                            "unknown SC64 command: {}",
                            char::from_u32(value & mask).unwrap_or('?')
                        );
                    }
                }
            }
        }
        _ => {
            eprintln!(
                "unknown SC64 write reg {reg} at {address:#x} value {}",
                char::from_u32(value & mask).unwrap_or('?')
            );
        }
    }
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    if address & 0x2000 != 0 {
        let masked_address = address as usize & SC64_EEPROM_MASK;
        device::memory::read_u32_be_at(&device.ui.storage.saves.eeprom.data, masked_address)
    } else {
        let masked_address = address as usize & SC64_BUFFER_MASK;
        device::memory::read_u32_be_at(&device.cart.sc64.buffer, masked_address)
    }
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if address & 0x2000 != 0 {
        let masked_address = address as usize & SC64_EEPROM_MASK;
        let mut data =
            device::memory::read_u32_be_at(&device.ui.storage.saves.eeprom.data, masked_address);
        device::memory::masked_write_32(&mut data, value, mask);
        device::memory::write_u32_be_at(
            &mut device.ui.storage.saves.eeprom.data,
            masked_address,
            data,
        );
        ui::storage::schedule_save(device, ui::storage::SaveTypes::Eeprom4k);
    } else {
        let masked_address = address as usize & SC64_BUFFER_MASK;
        let mut data = device::memory::read_u32_be_at(&device.cart.sc64.buffer, masked_address);
        device::memory::masked_write_32(&mut data, value, mask);
        device::memory::write_u32_be_at(&mut device.cart.sc64.buffer, masked_address, data);
    }
}

pub fn dma_read(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    let buffer = if cart_addr & 0x2000 != 0 {
        cart_addr &= SC64_EEPROM_MASK as u32;
        ui::storage::schedule_save(device, ui::storage::SaveTypes::Eeprom4k);
        &mut device.ui.storage.saves.eeprom.data
    } else {
        cart_addr &= SC64_BUFFER_MASK as u32;
        &mut device.cart.sc64.buffer
    };
    let mut i = dram_addr;
    let mut j = cart_addr;

    while i < dram_addr + length {
        *buffer.get_mut(j as usize).unwrap_or(&mut 0) = *device
            .rdram
            .mem
            .get(i as usize ^ device.byte_swap)
            .unwrap_or(&0);
        i += 1;
        j += 1;
    }

    device::pi::calculate_cycles(device, 1, length)
}

pub fn dma_write(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    let buffer = if cart_addr & 0x2000 != 0 {
        cart_addr &= SC64_EEPROM_MASK as u32;
        &device.ui.storage.saves.eeprom.data
    } else {
        cart_addr &= SC64_BUFFER_MASK as u32;
        &device.cart.sc64.buffer
    };
    let mut i = dram_addr;
    let mut j = cart_addr;

    while i < dram_addr + length {
        *device
            .rdram
            .mem
            .get_mut(i as usize ^ device.byte_swap)
            .unwrap_or(&mut 0) = *buffer.get(j as usize).unwrap_or(&0);
        i += 1;
        j += 1;
    }
    device::pi::calculate_cycles(device, 1, length)
}

#[cfg(test)]
mod tests {
    #[test]
    fn unknown_sc64_register_read_returns_zero() {
        let mut device = *crate::device::Device::new(false);
        let value = super::read_regs(
            &mut device,
            0x1FFF_0100,
            crate::device::memory::AccessSize::Word,
        );
        assert_eq!(value, 0);
    }

    #[test]
    fn sc64_get_config_clamps_out_of_range_index() {
        let mut device = *crate::device::Device::new(false);
        device.cart.sc64.regs_locked = false;
        device.cart.sc64.regs[super::SC64_DATA0_REG] = super::SC64_CFG_COUNT as u32 + 4;
        super::write_regs(
            &mut device,
            0x1FFF_0000,
            u32::from('c'),
            0xFF,
        );
        assert_eq!(device.cart.sc64.regs[super::SC64_DATA1_REG], 0);
    }

    #[test]
    fn sc64_buffer_read_is_bounds_safe() {
        let mut device = *crate::device::Device::new(false);
        device.cart.sc64.buffer = vec![0; 0x2000];
        let value = super::read_mem(
            &mut device,
            0x1FFE_3000,
            crate::device::memory::AccessSize::Word,
        );
        assert_eq!(value, 0);
    }
}
