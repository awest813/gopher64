// RDRAM register initialization adapted from rasky/small64 (MIT License).
// Copyright (c) 2025 Giovanni Bajo. See THIRD_PARTY_NOTICES.md.

use crate::device;

const RI_CONFIG_AUTO_CALIBRATION: u32 = 0x40;
const RI_SELECT_RX_TX: u32 = 0x14;
const RI_MODE_RESET: u32 = 0;
const RI_MODE_STANDARD: u32 = 0x8 | 0x4 | 0x2;
const RI_REFRESH_AUTO: u32 = 1 << 17;
const RI_REFRESH_OPTIMIZE: u32 = 1 << 18;

const RDRAM_REG_DEVICE_ID: usize = 1;
const RDRAM_REG_DELAY: usize = 2;
const RDRAM_REG_MODE: usize = 3;
const RDRAM_REG_REF_ROW: usize = 5;
const RDRAM_REG_RAS_INTERVAL: usize = 6;

const RDRAM_REG_MODE_DE: u32 = 1 << 25;
const RDRAM_REG_MODE_AS: u32 = 1 << 26;
const INITID: u32 = 0x1F;
const DEFAULT_CURRENT_CALIBRATION: u32 = 0x18;

const MI_SET_INIT: u32 = 1 << 8;
const MI_INIT_LENGTH_16: u32 = 15;

fn bit(x: u32, n: u32) -> u32 {
    (x >> n) & 1
}

fn bits(x: u32, start: u32, end: u32) -> u32 {
    (x >> start) & ((1 << (end - start + 1)) - 1)
}

fn bitswap5(x: u32) -> u32 {
    (bit(x, 0) << 4) | (bit(x, 1) << 3) | (bit(x, 2) << 2) | (bit(x, 3) << 1) | (bit(x, 4) << 0)
}

fn rot16(x: u32) -> u32 {
    ((x & 0xFFFF0000) >> 16) | ((x & 0xFFFF) << 16)
}

fn rdram_reg_mode_cc(cc: u32) -> u32 {
    let cc = cc ^ 0x3F;
    (bit(cc, 0) << 6)
        | (bit(cc, 1) << 14)
        | (bit(cc, 2) << 22)
        | (bit(cc, 3) << 7)
        | (bit(cc, 4) << 15)
        | (bit(cc, 5) << 23)
}

fn rdram_reg_delay_make(ack_win_delay: u32, read_delay: u32, ack_delay: u32, write_delay: u32) -> u32 {
    (((ack_win_delay & 7) << 3) << 24)
        | (((read_delay & 7) << 3) << 16)
        | (((ack_delay & 3) << 3) << 8)
        | (((write_delay & 7) << 3) << 0)
}

fn rdram_reg_device_id_make(chip_id: u32) -> u32 {
    (bits(chip_id, 0, 5) << 26)
        | (bits(chip_id, 6, 6) << 23)
        | (bits(chip_id, 7, 14) << 8)
        | (bits(chip_id, 15, 15) << 7)
}

fn rdram_reg_rasinterval_make(
    row_precharge: u32,
    row_sense: u32,
    row_imp_restore: u32,
    row_exp_restore: u32,
) -> u32 {
    (bitswap5(row_precharge) << 24)
        | (bitswap5(row_sense) << 16)
        | (bitswap5(row_imp_restore) << 8)
        | (bitswap5(row_exp_restore) << 0)
}

fn ri_refresh_value(num_banks: u32) -> u32 {
    RI_REFRESH_AUTO
        | RI_REFRESH_OPTIMIZE
        | (52 & 0xFF)
        | ((54 & 0xFF) << 8)
        | (((1 << num_banks) - 1) << 19)
}

fn rdram_mode_value(current_calibration: u32) -> u32 {
    RDRAM_REG_MODE_DE | RDRAM_REG_MODE_AS | 0x4000_0000 | rdram_reg_mode_cc(current_calibration)
}

fn num_rdram_banks(rdram_size: u32) -> u32 {
    if rdram_size >= 0x800000 {
        4
    } else {
        2
    }
}

fn apply_broadcast_rdram_regs(device: &mut device::Device, mode: u32) {
    let delay = rot16(rdram_reg_delay_make(5, 7, 3, 1));
    let device_id = rdram_reg_device_id_make(INITID);
    let ras_interval = rdram_reg_rasinterval_make(1, 7, 10, 4);

    for chip in device.rdram.regs.iter_mut() {
        chip[RDRAM_REG_DELAY] = delay;
        chip[RDRAM_REG_DEVICE_ID] = device_id;
        chip[RDRAM_REG_REF_ROW] = 0;
        chip[RDRAM_REG_MODE] = mode;
        chip[RDRAM_REG_RAS_INTERVAL] = ras_interval;
    }
}

/// Initialize RI/RDRAM registers using the compact sequence from small64.
pub fn init_registers(device: &mut device::Device) {
    let num_banks = num_rdram_banks(device.rdram.size);
    let mode = rdram_mode_value(DEFAULT_CURRENT_CALIBRATION);

    device.ri.regs[device::ri::RI_CONFIG_REG] = RI_CONFIG_AUTO_CALIBRATION;
    device.ri.regs[device::ri::RI_SELECT_REG] = RI_SELECT_RX_TX;
    device.ri.regs[device::ri::RI_REFRESH_REG] = ri_refresh_value(num_banks);
    device.ri.regs[device::ri::RI_MODE_REG] = RI_MODE_STANDARD;
    device.ri.ram_init = true;

    apply_broadcast_rdram_regs(device, mode);

    let chip_ids: &[u32] = if num_banks >= 4 {
        &[0, 2, 4, 6]
    } else {
        &[0, 2]
    };
    for &chip_id in chip_ids {
        let chip_index = (chip_id / 2) as usize;
        if chip_index < device.rdram.regs.len() {
            device.rdram.regs[chip_index][RDRAM_REG_DEVICE_ID] =
                rdram_reg_device_id_make(chip_id);
        }
    }

    device.mi.regs[device::mi::MI_INIT_MODE_REG] =
        device::mi::MI_INIT_MODE | MI_SET_INIT | MI_INIT_LENGTH_16;

    // IPL3 stores the detected memory size at these locations for libdragon/homebrew.
    device
        .rdram
        .mem
        .get_mut(0x318..0x318 + 4)
        .unwrap_or(&mut [0; 4])
        .copy_from_slice(&device.rdram.size.to_ne_bytes());
    device
        .rdram
        .mem
        .get_mut(0x3f0..0x3f0 + 4)
        .unwrap_or(&mut [0; 4])
        .copy_from_slice(&device.rdram.size.to_ne_bytes());
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_CURRENT_CALIBRATION, INITID, rdram_mode_value, rdram_reg_delay_make,
        rdram_reg_device_id_make, rdram_reg_mode_cc, rdram_reg_rasinterval_make, ri_refresh_value,
        rot16,
    };

    #[test]
    fn rdram_reg_mode_cc_scrambles_calibration_nibble() {
        assert_ne!(rdram_reg_mode_cc(0x18), rdram_reg_mode_cc(0x20));
    }

    #[test]
    fn rdram_reg_delay_make_rotates() {
        let delay = rot16(rdram_reg_delay_make(5, 7, 3, 1));
        assert_eq!(delay, 0x1808_2838);
    }

    #[test]
    fn rdram_reg_device_id_make_initid() {
        assert_eq!(rdram_reg_device_id_make(INITID), rdram_reg_device_id_make(0x1F));
    }

    #[test]
    fn rdram_mode_value_uses_default_calibration() {
        let mode = rdram_mode_value(DEFAULT_CURRENT_CALIBRATION);
        assert_ne!(mode & rdram_reg_mode_cc(DEFAULT_CURRENT_CALIBRATION), 0);
    }

    #[test]
    fn ri_refresh_value_for_two_banks() {
        let refresh = ri_refresh_value(2);
        assert_eq!(refresh & 0xFF, 52);
        assert_eq!((refresh >> 8) & 0xFF, 54);
        assert_eq!((refresh >> 19) & 0xF, 3);
    }

    #[test]
    fn rdram_reg_rasinterval_make_encodes_rows() {
        let value = rdram_reg_rasinterval_make(1, 7, 10, 4);
        assert_eq!(value, 0x101C_0A04);
    }
}
