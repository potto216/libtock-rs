#![no_std]

use core::cell::Cell;
use core::fmt;
use core::marker::PhantomData;
use libtock_platform as platform;
use libtock_platform::allow_ro::AllowRo;
use libtock_platform::allow_rw::AllowRw;
use libtock_platform::share;
use libtock_platform::subscribe::Subscribe;
use libtock_platform::{DefaultConfig, ErrorCode, Syscalls};

/// Byte-oriented UART syscall wrapper.
///
/// This uses the same syscall ABI as Tock's console capsule, but the driver
/// number is const-generic so a board can expose an additional console-style
/// capsule on a physically separate UART while keeping `Console` for logs.
pub struct Uart<S: Syscalls, const DRIVER_NUM: u32, C: Config = DefaultConfig>(S, C);

impl<S: Syscalls, const DRIVER_NUM: u32, C: Config> Uart<S, DRIVER_NUM, C> {
    /// Checks whether a byte-stream UART driver is available at `DRIVER_NUM`.
    pub fn exists() -> bool {
        S::command(DRIVER_NUM, command::EXISTS, 0, 0).is_success()
    }

    /// Writes `buffer` to the UART and waits for completion.
    pub fn write(buffer: &[u8]) -> Result<(), ErrorCode> {
        let called: Cell<Option<(u32,)>> = Cell::new(None);
        share::scope::<
            (
                AllowRo<_, DRIVER_NUM, { allow_ro::WRITE }>,
                Subscribe<_, DRIVER_NUM, { subscribe::WRITE }>,
            ),
            _,
            _,
        >(|handle| {
            let (allow_ro, subscribe) = handle.split();

            S::allow_ro::<C, DRIVER_NUM, { allow_ro::WRITE }>(allow_ro, buffer)?;
            S::subscribe::<_, _, C, DRIVER_NUM, { subscribe::WRITE }>(subscribe, &called)?;
            S::command(DRIVER_NUM, command::WRITE, buffer.len() as u32, 0)
                .to_result::<(), ErrorCode>()?;

            loop {
                S::yield_wait();
                if called.get().is_some() {
                    return Ok(());
                }
            }
        })
    }

    /// Reads bytes into `buffer` and waits for completion.
    ///
    /// Returns the number of bytes read plus the operation result.
    pub fn read(buffer: &mut [u8]) -> (usize, Result<(), ErrorCode>) {
        let called: Cell<Option<(u32, u32)>> = Cell::new(None);
        let len = buffer.len();
        let mut bytes_received = 0;
        let result = share::scope::<
            (
                AllowRw<_, DRIVER_NUM, { allow_rw::READ }>,
                Subscribe<_, DRIVER_NUM, { subscribe::READ }>,
            ),
            _,
            _,
        >(|handle| {
            let (allow_rw, subscribe) = handle.split();
            S::allow_rw::<C, DRIVER_NUM, { allow_rw::READ }>(allow_rw, buffer)?;
            S::subscribe::<_, _, C, DRIVER_NUM, { subscribe::READ }>(subscribe, &called)?;
            S::command(DRIVER_NUM, command::READ, len as u32, 0).to_result::<(), ErrorCode>()?;

            loop {
                S::yield_wait();
                if let Some((status, count)) = called.get() {
                    bytes_received = count as usize;
                    return match status {
                        0 => Ok(()),
                        e_status => Err(e_status.try_into().unwrap_or(ErrorCode::Fail)),
                    };
                }
            }
        });

        (bytes_received, result)
    }

    /// Reads until `buffer` is full.
    pub fn read_exact(buffer: &mut [u8]) -> Result<(), ErrorCode> {
        let mut offset = 0;

        while offset < buffer.len() {
            let (count, result) = Self::read(&mut buffer[offset..]);
            result?;

            if count == 0 {
                return Err(ErrorCode::Fail);
            }

            offset += count;
        }

        Ok(())
    }

    pub fn writer() -> UartWriter<S, DRIVER_NUM> {
        UartWriter {
            syscalls: Default::default(),
        }
    }
}

pub struct UartWriter<S: Syscalls, const DRIVER_NUM: u32> {
    syscalls: PhantomData<S>,
}

impl<S: Syscalls, const DRIVER_NUM: u32> fmt::Write for UartWriter<S, DRIVER_NUM> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        Uart::<S, DRIVER_NUM>::write(s.as_bytes()).map_err(|_e| fmt::Error)
    }
}

/// System call configuration trait for `Uart`.
pub trait Config:
    platform::allow_ro::Config + platform::allow_rw::Config + platform::subscribe::Config
{
}
impl<T: platform::allow_ro::Config + platform::allow_rw::Config + platform::subscribe::Config>
    Config for T
{
}

#[cfg(test)]
mod tests;

#[allow(unused)]
mod command {
    pub const EXISTS: u32 = 0;
    pub const WRITE: u32 = 1;
    pub const READ: u32 = 2;
    pub const ABORT: u32 = 3;
}

#[allow(unused)]
mod subscribe {
    pub const WRITE: u32 = 1;
    pub const READ: u32 = 2;
}

mod allow_ro {
    pub const WRITE: u32 = 1;
}

mod allow_rw {
    pub const READ: u32 = 1;
}
