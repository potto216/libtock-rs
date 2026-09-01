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
use libtock::platform::ErrorCode;
use libtock::runtime::{set_main, stack_size};

set_main! {main}
stack_size! {0x600}

const TRANSMIT_INTERVAL_MS: u32 = 1000;

const FRAME_TYPE_DATA: u16 = 1;
const PAN_ID_COMPRESSION: u16 = 1 << 6;
const ADDRESS_MODE_SHORT: u16 = 2;
const FRAME_VERSION_IEEE_802154_2006: u16 = 1;

// Ref: Section 7.2.2.2 Frame Type field, Table 7-3 addressing modes, and Table 7-4
// Frame Version field (IEEE 802.15.4-2020)
const FCF_DATA: u16 = FRAME_TYPE_DATA
    | PAN_ID_COMPRESSION
    | (ADDRESS_MODE_SHORT << 10)
    | (FRAME_VERSION_IEEE_802154_2006 << 12)
    | (ADDRESS_MODE_SHORT << 14);

// Destination address configuration. 0xffff broadcasts to every device in the
// PAN; replace it with a specific short address to send a unicast frame.
const DESTINATION_ADDRESS: u16 = 0xffff;

// Ref: Section 7.2.2 MAC Frame Format (IEEE 802.15.4-2020)
// If the address modes above change, update these header offsets and MAC_HEADER_LEN.
const FRAME_CONTROL_OFFSET: usize = 0;
const SEQUENCE_NUMBER_OFFSET: usize = 2;
const DESTINATION_PAN_ID_OFFSET: usize = 3;
const DESTINATION_ADDRESS_OFFSET: usize = 5;
const SOURCE_ADDRESS_OFFSET: usize = 7;
const MAC_HEADER_LEN: usize = 9;

// Payload configuration: change PAYLOAD_PREFIX or populate_payload() to use
// different fixed-length payload data.
const COUNTER_LEN: usize = core::mem::size_of::<u16>();
const PAYLOAD_PREFIX: &[u8] = b"beacon ";
const PAYLOAD_PREFIX_LEN: usize = PAYLOAD_PREFIX.len();
const PAYLOAD_LEN: usize = PAYLOAD_PREFIX_LEN + COUNTER_LEN;

// An IEEE 802.15.4 frame can contain at most 127 bytes, including the two-byte
// MAC footer added by the radio. This example supplies the header and payload.
const MAX_MAC_FRAME_LEN: usize = 127;
const MAC_FOOTER_LEN: usize = 2;
const MAX_PAYLOAD_LEN: usize = MAX_MAC_FRAME_LEN - MAC_FOOTER_LEN - MAC_HEADER_LEN;
const FRAME_LEN: usize = {
    assert!(
        PAYLOAD_LEN <= MAX_PAYLOAD_LEN,
        "PAYLOAD_LEN is too large for an IEEE 802.15.4 frame"
    );
    MAC_HEADER_LEN + PAYLOAD_LEN
};

// Source address configuration: change these values for the transmitting device.
// The raw frame uses PAN_ID and SHORT_ADDRESS; LONG_ADDRESS configures the radio.
pub const PAN_ID: u16 = 0xcafe;
pub const SHORT_ADDRESS: u16 = 0x1001;
pub const LONG_ADDRESS: u64 = 0xdeaddad;
pub const CHANNEL: u8 = 11;
pub const TX_POWER: i8 = 4;

/// Initializes the header in an exact-length data frame.
fn initialize_data_frame(frame: &mut [u8; FRAME_LEN]) {
    frame[FRAME_CONTROL_OFFSET..SEQUENCE_NUMBER_OFFSET].copy_from_slice(&FCF_DATA.to_le_bytes());
    frame[DESTINATION_PAN_ID_OFFSET..DESTINATION_ADDRESS_OFFSET]
        .copy_from_slice(&PAN_ID.to_le_bytes());
    frame[DESTINATION_ADDRESS_OFFSET..SOURCE_ADDRESS_OFFSET]
        .copy_from_slice(&DESTINATION_ADDRESS.to_le_bytes());
    frame[SOURCE_ADDRESS_OFFSET..MAC_HEADER_LEN].copy_from_slice(&SHORT_ADDRESS.to_le_bytes());
}

/// Creates every byte of the fixed-length payload for one transmission.
///
/// Modify this function and the payload constants above to generate different
/// payload contents. Returning `[u8; PAYLOAD_LEN]` keeps its size fixed.
fn populate_payload(counter: u16) -> [u8; PAYLOAD_LEN] {
    let mut payload = [0_u8; PAYLOAD_LEN];
    payload[..PAYLOAD_PREFIX_LEN].copy_from_slice(PAYLOAD_PREFIX);
    payload[PAYLOAD_PREFIX_LEN..].copy_from_slice(&counter.to_be_bytes());
    payload
}

/// Transmits only a frame whose array length is exactly [`FRAME_LEN`].
fn transmit_data_frame(frame: &[u8; FRAME_LEN]) -> Result<(), ErrorCode> {
    Ieee802154::transmit_frame_raw(frame)
}

fn main() {
    // Configure the radio
    writeln!(Console::writer(), "Configuring IEEE 802.15.4 radio...").unwrap();

    Ieee802154::set_pan(PAN_ID);
    writeln!(Console::writer(), "Set PAN to {PAN_ID:#06x}").unwrap();

    Ieee802154::set_address_short(SHORT_ADDRESS);
    writeln!(
        Console::writer(),
        "Set short address to {SHORT_ADDRESS:#06x}"
    )
    .unwrap();

    Ieee802154::set_address_long(LONG_ADDRESS);
    writeln!(
        Console::writer(),
        "Set long address to {LONG_ADDRESS:#018x}"
    )
    .unwrap();

    Ieee802154::set_tx_power(TX_POWER).unwrap();
    writeln!(Console::writer(), "Set TX power to {TX_POWER}").unwrap();

    Ieee802154::set_channel(CHANNEL).unwrap();
    writeln!(Console::writer(), "Set channel to {CHANNEL}").unwrap();

    // Don't forget to commit the config!
    Ieee802154::commit_config();
    writeln!(Console::writer(), "Committed radio configuration!").unwrap();

    // Turn the radio on
    Ieee802154::radio_on().unwrap();
    assert!(Ieee802154::is_on());
    writeln!(Console::writer(), "Radio is on!").unwrap();

    // This array has exactly the length checked against MAX_MAC_FRAME_LEN above,
    // so passing the whole array cannot accidentally transmit unused buffer bytes.
    let mut tx_frame = [0_u8; FRAME_LEN];
    initialize_data_frame(&mut tx_frame);

    let mut sequence = 0_u8;
    let mut counter = 0_u16;

    loop {
        Alarm::sleep_for(Milliseconds(TRANSMIT_INTERVAL_MS)).unwrap();

        tx_frame[SEQUENCE_NUMBER_OFFSET] = sequence;

        // Replace the entire payload on every iteration.
        let payload = populate_payload(counter);
        tx_frame[MAC_HEADER_LEN..].copy_from_slice(&payload);

        if let Err(error) = transmit_data_frame(&tx_frame) {
            writeln!(Console::writer(), "TX failed: {error:?}").unwrap();
            // Retry the same sequence number and counter after the next interval.
            continue;
        }

        writeln!(
            Console::writer(),
            "TX data: src={SHORT_ADDRESS:#06x}, dst={DESTINATION_ADDRESS:#06x}, sequence={sequence}, count={counter}",
        )
        .unwrap();

        sequence = sequence.wrapping_add(1);
        counter = counter.wrapping_add(1);
    }
}
