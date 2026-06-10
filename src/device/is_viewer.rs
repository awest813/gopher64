// ISViewer host interface adapted from lemmy-64/n64-systemtest (MIT License).
// Spec: https://github.com/lemmy-64/n64-systemtest#isviewer
// Copyright (c) 2021 lemmy-64. See THIRD_PARTY_NOTICES.md.

use crate::device;

const IS_VIEWER_MASK: usize = 0xFFFF;
/// Length register at 0xB3FF0014; writing triggers a host print of the text buffer.
const WRITE_LEN_OFFSET: usize = 0x14;
/// Text buffer at 0xB3FF0020 through 0xB3FF0220 (512 bytes).
const BUF_OFFSET: usize = 0x20;
const BUF_SIZE: usize = 0x200;

fn flush_text_buffer(device: &mut device::Device, length: usize) {
    let length = length.min(BUF_SIZE);
    let end = BUF_OFFSET.saturating_add(length);
    let slice = device
        .cart
        .is_viewer_buffer
        .get(BUF_OFFSET..end)
        .unwrap_or(&[]);
    if let Ok(text) = std::str::from_utf8(slice) {
        print!("{text}");
    } else {
        print!("{}", String::from_utf8_lossy(slice));
    }
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let masked_address = address as usize & IS_VIEWER_MASK;
    device::memory::read_u32_be_at(&device.cart.is_viewer_buffer, masked_address)
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let masked_address = address as usize & IS_VIEWER_MASK;
    if masked_address == WRITE_LEN_OFFSET {
        let mut length_reg =
            device::memory::read_u32_be_at(&device.cart.is_viewer_buffer, WRITE_LEN_OFFSET);
        device::memory::masked_write_32(&mut length_reg, value, mask);
        device::memory::write_u32_be_at(
            &mut device.cart.is_viewer_buffer,
            WRITE_LEN_OFFSET,
            length_reg,
        );
        flush_text_buffer(device, length_reg as usize);
        return;
    }

    let mut data =
        device::memory::read_u32_be_at(&device.cart.is_viewer_buffer, masked_address);
    device::memory::masked_write_32(&mut data, value, mask);
    device::memory::write_u32_be_at(&mut device.cart.is_viewer_buffer, masked_address, data);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_viewer_phys(offset: usize) -> u64 {
        0x13FF_0000 + offset as u64
    }

    #[test]
    fn is_viewer_detect_roundtrip() {
        let mut device = *crate::device::Device::new(false);
        let magic = 0x1234_5678u32;

        super::write_mem(
            &mut device,
            is_viewer_phys(BUF_OFFSET),
            magic,
            0xFFFF_FFFF,
        );
        let readback = super::read_mem(
            &mut device,
            is_viewer_phys(BUF_OFFSET),
            crate::device::memory::AccessSize::Word,
        );
        assert_eq!(readback, magic);
    }

    #[test]
    fn is_viewer_length_write_stores_register() {
        let mut device = *crate::device::Device::new(false);
        let text = b"pass";
        device.cart.is_viewer_buffer[BUF_OFFSET..BUF_OFFSET + text.len()].copy_from_slice(text);

        super::write_mem(
            &mut device,
            is_viewer_phys(WRITE_LEN_OFFSET),
            text.len() as u32,
            0xFFFF_FFFF,
        );

        assert_eq!(
            device::memory::read_u32_be_at(&device.cart.is_viewer_buffer, WRITE_LEN_OFFSET),
            text.len() as u32
        );
    }

    #[test]
    fn is_viewer_length_clamps_to_buffer_size() {
        let mut device = *crate::device::Device::new(false);
        device.cart.is_viewer_buffer[BUF_OFFSET] = b'x';

        super::write_mem(
            &mut device,
            is_viewer_phys(WRITE_LEN_OFFSET),
            BUF_SIZE as u32 + 64,
            0xFFFF_FFFF,
        );

        assert_eq!(
            device::memory::read_u32_be_at(&device.cart.is_viewer_buffer, WRITE_LEN_OFFSET),
            (BUF_SIZE + 64) as u32
        );
    }

    #[test]
    fn is_viewer_read_out_of_range_returns_zero() {
        let device = *crate::device::Device::new(false);
        let value = device::memory::read_u32_be_at(
            &device.cart.is_viewer_buffer,
            device.cart.is_viewer_buffer.len(),
        );
        assert_eq!(value, 0);
    }

    #[test]
    fn is_viewer_byte_write_to_buffer() {
        let mut device = *crate::device::Device::new(false);

        super::write_mem(
            &mut device,
            is_viewer_phys(BUF_OFFSET + 1),
            0x42,
            0x0000_FF00,
        );

        assert_eq!(device.cart.is_viewer_buffer[BUF_OFFSET + 1], 0x42);
    }
}
