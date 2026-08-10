//! An example showing use of IEEE 802.15.4 networking.
//! It infinitely sends a frame with a constantly incremented counter.
//!
//! The kernel contains a standard and phy 15.4 driver. This example
//! expects the kernel to be configured with the phy 15.4 driver to
//! allow direct access to the radio and the ability to send "raw"
//! frames. An example board file using this driver is provided at
//! `boards/tutorials/nrf52840dk-thread-tutorial`.
//!
//! "No Support" Errors for setting the channel/tx power are a telltale
//! sign that the kernel is not configured with the phy 15.4 driver.

#![no_main]
#![no_std]
use core::fmt::Write;

use libtock::alarm::{Alarm, Milliseconds};
use libtock::console::Console;
use libtock::ieee802154::Ieee802154;
use libtock::runtime::{set_main, stack_size};

set_main! {main}
stack_size! {0x600}

const TRANSMIT_INTERVAL_MS: u32 = 1000;

const FCF_BROADCAST_DATA: u16 = 0x9841;
const BROADCAST_ADDRESS: u16 = 0xffff;
const MAC_HEADER_LEN: usize = 9;

const BEACON_TAG: &[u8; 7] = b"beacon ";
const COUNTER_LEN: usize = 2;
const FRAME_LEN: usize = MAC_HEADER_LEN + BEACON_TAG.len() + COUNTER_LEN;
const COUNTER_OFFSET: usize = MAC_HEADER_LEN + BEACON_TAG.len();

pub const DEVICE_NAME: &str = "board-a";
pub const PAN_ID: u16 = 0xcafe;
pub const SHORT_ADDRESS: u16 = 0x1001;
pub const LONG_ADDRESS: u64 = 0xdeaddad;
pub const CHANNEL: u8 = 11;
pub const TX_POWER: i8 = 4;

fn main() {
    // Configure the radio
    let pan: u16 = PAN_ID;
    let addr_short: u16 = SHORT_ADDRESS;
    let addr_long: u64 = LONG_ADDRESS;
    let tx_power: i8 = TX_POWER;
    let channel: u8 = CHANNEL;

    writeln!(Console::writer(), "Configuring IEEE 802.15.4 radio...\n").unwrap();

    Ieee802154::set_pan(pan);
    writeln!(Console::writer(), "Set PAN to {:#06x}\n", pan).unwrap();

    Ieee802154::set_address_short(addr_short);
    writeln!(
        Console::writer(),
        "Set short address to {:#06x}\n",
        addr_short
    )
    .unwrap();

    Ieee802154::set_address_long(addr_long);
    writeln!(
        Console::writer(),
        "Set long address to {:#018x}\n",
        addr_long
    )
    .unwrap();

    Ieee802154::set_tx_power(tx_power).unwrap();
    writeln!(Console::writer(), "Set TX power to {}\n", tx_power).unwrap();

    Ieee802154::set_channel(channel).unwrap();
    writeln!(Console::writer(), "Set channel to {}\n", channel).unwrap();

    // Don't forget to commit the config!
    Ieee802154::commit_config();
    writeln!(Console::writer(), "Committed radio configuration!\n").unwrap();

    // Turn the radio on
    Ieee802154::radio_on().unwrap();
    assert!(Ieee802154::is_on());
    writeln!(Console::writer(), "Radio is on!\n").unwrap();

    let mut tx_frame = [0_u8; FRAME_LEN];
    tx_frame[0..2].copy_from_slice(&FCF_BROADCAST_DATA.to_le_bytes());
    tx_frame[3..5].copy_from_slice(&PAN_ID.to_le_bytes());
    tx_frame[5..7].copy_from_slice(&BROADCAST_ADDRESS.to_le_bytes());
    tx_frame[7..9].copy_from_slice(&SHORT_ADDRESS.to_le_bytes());
    tx_frame[MAC_HEADER_LEN..COUNTER_OFFSET].copy_from_slice(BEACON_TAG);

    let mut sequence = 0_u8;
    let mut counter = 0_u16;

    loop {
        Alarm::sleep_for(Milliseconds(TRANSMIT_INTERVAL_MS)).unwrap();

        tx_frame[2] = sequence;
        tx_frame[COUNTER_OFFSET..FRAME_LEN].copy_from_slice(&counter.to_be_bytes());

        match Ieee802154::transmit_frame_raw(&tx_frame) {
            Ok(()) => {}
            Err(error) => {
                writeln!(Console::writer(), "TX failed: {:?}", error).unwrap();
                continue;
            }
        }

        writeln!(
            Console::writer(),
            "TX broadcast: src={:#06x}, sequence={}, count={}",
            SHORT_ADDRESS,
            sequence,
            counter,
        )
        .unwrap();

        sequence = sequence.wrapping_add(1);
        counter = counter.wrapping_add(1);
    }
}
