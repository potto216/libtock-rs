use super::*;
use core::fmt::Write;
use libtock_unittest::fake;

const CONSOLE_ABI_DRIVER_NUM: u32 = 0x1;
type TestUart = Uart<fake::Syscalls, CONSOLE_ABI_DRIVER_NUM>;

#[test]
fn no_driver() {
    let _kernel = fake::Kernel::new();
    assert!(!TestUart::exists());
}

#[test]
fn exists() {
    let kernel = fake::Kernel::new();
    let driver = fake::Console::new();
    kernel.add_driver(&driver);

    assert!(TestUart::exists());
}

#[test]
fn write_bytes() {
    let kernel = fake::Kernel::new();
    let driver = fake::Console::new();
    kernel.add_driver(&driver);

    TestUart::write(b"tx-bytes").unwrap();
    assert_eq!(driver.take_bytes(), b"tx-bytes");
}

#[test]
fn write_str() {
    let kernel = fake::Kernel::new();
    let driver = fake::Console::new();
    kernel.add_driver(&driver);

    write!(TestUart::writer(), "tx-str").unwrap();
    assert_eq!(driver.take_bytes(), b"tx-str");
}

#[test]
fn read_bytes() {
    let kernel = fake::Kernel::new();
    let driver = fake::Console::new_with_input(b"rx-bytes");
    kernel.add_driver(&driver);

    let mut buf = [0; 8];
    let (count, result) = TestUart::read(&mut buf);

    result.unwrap();
    assert_eq!(count, 8);
    assert_eq!(&buf, b"rx-bytes");
}

#[test]
fn read_exact_bytes() {
    let kernel = fake::Kernel::new();
    let driver = fake::Console::new_with_input(b"rx-bytes");
    kernel.add_driver(&driver);

    let mut buf = [0; 8];

    TestUart::read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"rx-bytes");
}
