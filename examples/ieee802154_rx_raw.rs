//! An example showing use of IEEE 802.15.4 networking.
//! It infinitely received a frame and prints its content to Console.
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
use libtock::console::Console;
use libtock::ieee802154::{Ieee802154, RxOperator as _, RxRingBuffer, RxSingleBufferOperator};
use libtock::runtime::{set_main, stack_size};

set_main! {main}
stack_size! {0x600}

const FRAME_TYPE_DATA: u8 = 1;
const ADDRESS_MODE_NONE: u8 = 0;
const ADDRESS_MODE_RESERVED: u8 = 1;
const ADDRESS_MODE_SHORT: u8 = 2;
const ADDRESS_MODE_EXTENDED: u8 = 3;
const FRAME_VERSION_IEEE_802154_2003: u8 = 0;
const FRAME_VERSION_IEEE_802154_2006: u8 = 1;
const FRAME_VERSION_IEEE_802154: u8 = 2;
const FRAME_VERSION_RESERVED: u8 = 3;

// Ref: Section 7.2.2.2 Frame Type field (IEEE 802.15.4-2020)
fn frame_type_name(frame_type: u8) -> &'static str {
    match frame_type {
        0 => "Beacon",
        FRAME_TYPE_DATA => "Data",
        2 => "Acknowledgment",
        3 => "MAC command",
        4 => "Reserved",
        5 => "Multipurpose",
        6 => "Fragment",
        7 => "Extended",
        _ => "Unknown",
    }
}

// Ref: Table 7-3 Destination Addressing Mode and Source Addressing Mode (IEEE 802.15.4-2020)
fn address_mode_name(address_mode: u8) -> &'static str {
    match address_mode {
        ADDRESS_MODE_NONE => "None",
        ADDRESS_MODE_RESERVED => "Reserved",
        ADDRESS_MODE_SHORT => "Short",
        ADDRESS_MODE_EXTENDED => "Extended",
        _ => "Unknown",
    }
}

// Ref: Table 7-4 Frame Version field (IEEE 802.15.4-2020)
fn frame_version_name(frame_version: u8) -> &'static str {
    match frame_version {
        FRAME_VERSION_IEEE_802154_2003 => "IEEE 802.15.4-2003",
        FRAME_VERSION_IEEE_802154_2006 => "IEEE 802.15.4-2006",
        FRAME_VERSION_IEEE_802154 => "IEEE 802.15.4",
        FRAME_VERSION_RESERVED => "Reserved",
        _ => "Unknown",
    }
}

fn read_u16(bytes: &[u8], offset: &mut usize) -> Option<u16> {
    let value = bytes.get(*offset..*offset + 2)?;
    *offset += 2;
    Some(u16::from_le_bytes([value[0], value[1]]))
}

fn print_address(
    label: &str,
    address_mode: u8,
    bytes: &[u8],
    offset: &mut usize,
) -> Result<(), ()> {
    match address_mode {
        ADDRESS_MODE_NONE => Ok(()),
        ADDRESS_MODE_SHORT => {
            let Some(address) = read_u16(bytes, offset) else {
                return Err(());
            };
            writeln!(Console::writer(), "{label} address: {address:#06x}").unwrap();
            Ok(())
        }
        ADDRESS_MODE_EXTENDED => {
            let Some(address) = bytes.get(*offset..*offset + 8) else {
                return Err(());
            };
            *offset += 8;
            let address = u64::from_le_bytes([
                address[0], address[1], address[2], address[3], address[4], address[5], address[6],
                address[7],
            ]);
            writeln!(Console::writer(), "{label} address: {address:#018x}").unwrap();
            Ok(())
        }
        _ => Err(()),
    }
}

fn print_payload(payload: &[u8]) {
    write!(Console::writer(), "Payload ({} bytes) hex: ", payload.len()).unwrap();
    for byte in payload {
        write!(Console::writer(), "{byte:02x}").unwrap();
    }
    writeln!(Console::writer()).unwrap();

    if let Ok(text) = core::str::from_utf8(payload) {
        writeln!(Console::writer(), "Payload UTF-8: {text:?}").unwrap();
    } else {
        writeln!(Console::writer(), "Payload is not valid UTF-8").unwrap();
    }
}

fn print_frame(frame: &[u8]) {
    if frame.len() < 2 {
        writeln!(
            Console::writer(),
            "Malformed frame: expected at least a 2-byte frame control field"
        )
        .unwrap();
        return;
    }
    // Ref: Section 7.2.2 MAC Frame Format (IEEE 802.15.4-2020)
    let frame_control = u16::from_le_bytes([frame[0], frame[1]]);
    let frame_type = (frame_control & 0x0007) as u8;
    let security_enabled = frame_control & 0x0008 != 0;
    let frame_pending = frame_control & 0x0010 != 0;
    let acknowledgment_requested = frame_control & 0x0020 != 0;
    let pan_id_compression = frame_control & 0x0040 != 0;
    // Bit 7 is reserved and should be set to 0
    let sequence_number_suppressed = frame_control & 0x0100 != 0;
    let information_elements_present = frame_control & 0x0200 != 0;
    let destination_address_mode = ((frame_control >> 10) & 0x0003) as u8;
    let frame_version = ((frame_control >> 12) & 0x0003) as u8;
    let source_address_mode = ((frame_control >> 14) & 0x0003) as u8;

    writeln!(Console::writer(), "Frame control: {frame_control:#06x}").unwrap();
    writeln!(
        Console::writer(),
        "Frame type: {} ({frame_type})",
        frame_type_name(frame_type)
    )
    .unwrap();
    writeln!(Console::writer(), "Security enabled: {security_enabled}").unwrap();
    writeln!(Console::writer(), "Frame pending: {frame_pending}").unwrap();
    writeln!(
        Console::writer(),
        "Acknowledgment requested: {acknowledgment_requested}"
    )
    .unwrap();
    writeln!(
        Console::writer(),
        "PAN ID compression: {pan_id_compression}"
    )
    .unwrap();
    writeln!(
        Console::writer(),
        "Sequence number suppression: {sequence_number_suppressed}"
    )
    .unwrap();
    writeln!(
        Console::writer(),
        "Information elements present: {information_elements_present}"
    )
    .unwrap();
    writeln!(
        Console::writer(),
        "Destination address mode: {} ({destination_address_mode})",
        address_mode_name(destination_address_mode)
    )
    .unwrap();
    writeln!(
        Console::writer(),
        "Frame version: {} ({frame_version})",
        frame_version_name(frame_version)
    )
    .unwrap();
    writeln!(
        Console::writer(),
        "Source address mode: {} ({source_address_mode})",
        address_mode_name(source_address_mode)
    )
    .unwrap();

    if frame_version > FRAME_VERSION_IEEE_802154 {
        writeln!(
            Console::writer(),
            "Address decoding for this frame version is not supported"
        )
        .unwrap();
        return;
    }

    let mut offset = 2;
    let sequence_number = if sequence_number_suppressed {
        None
    } else {
        let Some(sequence_number) = frame.get(offset).copied() else {
            writeln!(Console::writer(), "Malformed sequence number").unwrap();
            return;
        };
        offset += 1;
        Some(sequence_number)
    };

    if let Some(sequence_number) = sequence_number {
        writeln!(Console::writer(), "Sequence number: {sequence_number}").unwrap();
    } else {
        writeln!(Console::writer(), "Sequence number: suppressed").unwrap();
    }

    let destination_pan = if destination_address_mode != ADDRESS_MODE_NONE {
        let Some(pan) = read_u16(frame, &mut offset) else {
            writeln!(Console::writer(), "Malformed destination PAN ID").unwrap();
            return;
        };
        writeln!(Console::writer(), "Destination PAN ID: {pan:#06x}").unwrap();
        Some(pan)
    } else {
        None
    };

    if print_address("Destination", destination_address_mode, frame, &mut offset).is_err() {
        writeln!(Console::writer(), "Malformed destination address").unwrap();
        return;
    }

    if source_address_mode != ADDRESS_MODE_NONE {
        if pan_id_compression {
            if let Some(pan) = destination_pan {
                writeln!(Console::writer(), "Source PAN ID: {pan:#06x} (compressed)").unwrap();
            } else {
                writeln!(
                    Console::writer(),
                    "Malformed header: compressed source PAN ID has no destination PAN ID"
                )
                .unwrap();
                return;
            }
        } else {
            let Some(pan) = read_u16(frame, &mut offset) else {
                writeln!(Console::writer(), "Malformed source PAN ID").unwrap();
                return;
            };
            writeln!(Console::writer(), "Source PAN ID: {pan:#06x}").unwrap();
        }
    }

    if print_address("Source", source_address_mode, frame, &mut offset).is_err() {
        writeln!(Console::writer(), "Malformed source address").unwrap();
        return;
    }

    writeln!(Console::writer(), "MAC header length: {offset} bytes").unwrap();

    if frame_type == FRAME_TYPE_DATA {
        if security_enabled || information_elements_present {
            writeln!(
                Console::writer(),
                "Payload decoding for secured frames or frames with information elements is not supported"
            )
            .unwrap();
            return;
        }
        print_payload(&frame[offset..]);
    }
}

fn main() {
    // Configure the radio
    let pan: u16 = 0xcafe;
    let addr_short: u16 = 0xdead;
    let addr_long: u64 = 0x0dea_ddad;
    let tx_power: i8 = 4;
    let channel: u8 = 11;

    writeln!(Console::writer(), "Configuring IEEE 802.15.4 radio...").unwrap();

    Ieee802154::set_pan(pan);
    writeln!(Console::writer(), "Set PAN to {pan:#06x}").unwrap();

    Ieee802154::set_address_short(addr_short);
    writeln!(Console::writer(), "Set short address to {addr_short:#06x}").unwrap();

    Ieee802154::set_address_long(addr_long);
    writeln!(Console::writer(), "Set long address to {addr_long:#018x}").unwrap();

    Ieee802154::set_tx_power(tx_power).unwrap();
    writeln!(Console::writer(), "Set TX power to {tx_power}").unwrap();

    Ieee802154::set_channel(channel).unwrap();
    writeln!(Console::writer(), "Set channel to {channel}").unwrap();

    // Don't forget to commit the config!
    Ieee802154::commit_config();
    writeln!(Console::writer(), "Committed radio configuration!").unwrap();

    // Turn the radio on
    Ieee802154::radio_on().unwrap();
    assert!(Ieee802154::is_on());
    writeln!(Console::writer(), "Radio is on!").unwrap();

    let mut buf = RxRingBuffer::<2>::new();
    let mut operator = RxSingleBufferOperator::new(&mut buf);
    loop {
        let frame = operator.receive_frame().unwrap();
        let frame_len = usize::from(frame.payload_len).min(frame.body.len());

        writeln!(Console::writer(), "Received frame ({frame_len} bytes)").unwrap();
        print_frame(&frame.body[..frame_len]);
        writeln!(Console::writer()).unwrap(); // Print a newline to separate frames
    }
}
