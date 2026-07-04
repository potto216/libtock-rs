//! Demonstrates byte I/O on a physical UART while keeping console logs separate.
//!
//! `Console` remains the normal logging console, for example a USB CDC UART.
//! `TestUart` targets a second console-style syscall driver number that the
//! board must wire to a separate physical UART and its own capsule/grant state.
//!
//! Kernel-side shape:
//! - USB CDC logging: `ConsoleComponent::new(..., capsules_core::console::DRIVER_NUM, usb_uart_mux)`
//! - Physical test UART: `ConsoleComponent::new(..., PHYSICAL_UART_DRIVER_NUM, physical_uart_mux)`
//!
//! Update `PHYSICAL_UART_DRIVER_NUM` below to match the number your board uses.

#![no_main]
#![no_std]

use core::fmt::Write;
use libtock::console::Console;
use libtock::runtime::{set_main, stack_size};
use libtock::uart::Uart;

set_main! {main}
stack_size! {0x400}

const PHYSICAL_UART_DRIVER_NUM: u32 = 0x90002;
const TX_PAYLOAD: &[u8] = b"libtock-rs physical UART tx payload\r\n";
const RX_PAYLOAD_LEN: usize = 8;

type TestUart = Uart<PHYSICAL_UART_DRIVER_NUM>;

fn main() {
    let mut console = Console::writer();

    writeln!(console, "physical-uart: byte send/receive example\r").unwrap();
    writeln!(
        console,
        "physical-uart: using driver 0x{PHYSICAL_UART_DRIVER_NUM:x}; logs stay on Console\r"
    )
    .unwrap();

    if !TestUart::exists() {
        writeln!(
            console,
            "physical-uart: driver 0x{PHYSICAL_UART_DRIVER_NUM:x} is not present\r"
        )
        .unwrap();
        return;
    }

    writeln!(
        console,
        "physical-uart: sending {} payload bytes\r",
        TX_PAYLOAD.len()
    )
    .unwrap();

    if let Err(why) = TestUart::write(TX_PAYLOAD) {
        writeln!(console, "physical-uart: write failed {why:?}\r").unwrap();
        return;
    }

    writeln!(
        console,
        "physical-uart: waiting to receive {RX_PAYLOAD_LEN} bytes\r"
    )
    .unwrap();

    let mut rx_buf = [0; RX_PAYLOAD_LEN];
    match TestUart::read_exact(&mut rx_buf) {
        Ok(()) => {
            writeln!(
                console,
                "physical-uart: received {} bytes from test UART: {rx_buf:x?}\r",
                rx_buf.len()
            )
            .unwrap();
        }
        Err(why) => {
            writeln!(console, "physical-uart: read failed {why:?}\r").unwrap();
        }
    }
}
