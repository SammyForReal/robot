use anyhow::{Context, Result};
use nix::{
    fcntl,
    libc::{poll, pollfd, POLLIN},
    sys::stat::Mode,
    unistd,
};
use std::{os::fd::BorrowedFd, path::Path};

///Path to Hypervisor Console, needed to interact with the high-level APIs in OC2R.
pub const PATH: &str = "/dev/hvc0";
const TIMEOUT: i32 = 10000; // 10s
const BUFFER_CHUNK: usize = 1024;

///Represents the bus.
pub struct Bus {
    fd: i32,
    buffer: Vec<u8>,
}

impl Bus {
    ///Creates a new bus for the given path.
    pub fn new(path: &str) -> Result<Self> {
        let bus_path = Path::new(path);

        let fd = fcntl::open(bus_path, fcntl::OFlag::O_RDWR, Mode::empty())
            .context("Could not access the bus")?;

        Ok(Bus {
            fd,
            buffer: Vec::new(),
        })
    }

    ///Reads from the bus until the end has been reached.
    pub fn read_msg(&mut self) -> Result<&Vec<u8>> {
        // Check if we already have been reading before
        if !self.buffer.is_empty() {
            return Ok(&self.buffer);
        }

        // file descriptors we want to listen for (just one)
        let mut fds = [pollfd {
            fd: self.fd,
            events: POLLIN,
            revents: 0,
        }];

        let result = unsafe { poll(fds.as_mut_ptr(), fds.len() as u64, TIMEOUT) };

        // Ensure being successfull before reading
        if result == -1 {
            return Err(anyhow::anyhow!("Poll failed"));
        } else if result == 0 {
            return Err(anyhow::anyhow!("Poll reached timeout"));
        } else if fds[0].revents & POLLIN == 0 {
            return Err(anyhow::anyhow!("Poll returned nothing"));
        }

        // Ready to read: fetch data from the fd until EOF
        loop {
            // Read fd
            let mut buffer = vec![0; BUFFER_CHUNK];
            let bytes_read =
                unistd::read(self.fd, &mut buffer).context("Could not read from poll")?;

            if bytes_read == 0 {
                // Reached EOF
                break;
            }

            // Put chunk into buffer
            buffer.truncate(bytes_read);
            self.buffer.extend_from_slice(&buffer);

            if bytes_read < BUFFER_CHUNK {
                break;
            }
        }

        return Ok(&self.buffer);
    }

    /// Returns a borrowed file descriptor (`BorrowedFd`) tied to the lifetime of the `Bus` instance.
    ///
    /// # Safety
    ///
    /// The returned `BorrowedFd` must not outlive the `Bus` struct. Do not close
    /// the file descriptor or drop the `Bus` struct while the `BorrowedFd` is in use.
    ///
    /// If the `Bus` is dropped or it's file descriptor is closed before the `BorrowedFd` is done
    /// being used, this will result in undefined behavior.
    pub fn borrow_raw<'fd>(&'fd self) -> BorrowedFd<'fd> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        if let Err(err) = unistd::close(self.fd).context("Could not close the bus") {
            eprintln!("{}", err);
        }
    }
}

pub fn prepare() -> Result<Bus> {
    let mut bus = Bus::new(PATH).context("Could not create access to the bus")?;

    // List for test purposes
    unistd::write(bus.borrow_raw(), b" { \"type\": \"list\" } ")
        .context("Could not write to the bus")?;

    let result = bus
        .read_msg()
        .context("Could not read result from list call")?;

    println!("Result of list call:\n\n{:?}", result);

    Ok(bus)
}
