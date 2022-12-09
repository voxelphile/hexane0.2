#![feature(default_free_fn)]

use std::mem;
use std::net;
use std::default::default;
use std::result;

#[cfg(target_os = "windows")]
use windows::Win32::Networking::WinSock as win_sock;

pub trait Serializable {}

pub trait Deserializable {}

pub struct Send<T> {
    marker: std::marker::PhantomData<T>,
}

pub struct Recv<T> {
    marker: std::marker::PhantomData<T>,
}

#[derive(Debug)]
pub enum Error {
    CantOpen,
    AddrAlreadyInUse,
}

type Result<T> = result::Result<T, Error>;

pub enum SocketType {
    Stream,
    Datagram,
}

pub struct Socket {
    #[cfg(target_os = "linux")]
    handle: libc::c_int,
    #[cfg(target_os = "windows")]
    handle: win_sock::SOCKET, 
}

impl Socket {
    #[cfg(target_os = "windows")]
    pub fn open(ty: SocketType) -> Result<Self> {
        use win_sock::*;

        let version: u16 = (2 << 8) | 2;

        let mut wsa_data = mem::MaybeUninit::<WSADATA>::new(default());

        unsafe { WSAStartup(version, &mut wsa_data as *mut _ as *mut _); }

        let handle = match ty {
            SocketType::Stream => unsafe { socket(AF_INET.0 as _, SOCK_STREAM as _, IPPROTO_TCP.0) },
            SocketType::Datagram => unsafe { socket(AF_INET.0 as _, SOCK_DGRAM as _, IPPROTO_UDP.0) },
        };

        if handle == INVALID_SOCKET {
            Err(Error::CantOpen)?;
        }

        Ok(Self { handle })
    }
    

    #[cfg(target_os = "linux")]
    pub fn open(ty: SocketType) -> Result<Self> {
        use libc::*;

        let handle = match ty {
            SocketType::Stream => unsafe { socket(AF_INET, SOCK_STREAM, IPPROTO_TCP) },
            SocketType::Datagram => unsafe { socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP) },
        };

        if handle == -1 {
            Err(Error::CantOpen)?;
        }

        Ok(Self { handle })
    }

    pub fn close(self) {
    }

    pub fn bind(&mut self, addrs: impl net::ToSocketAddrs) -> Result<()> {
        #[cfg(target_os = "linux")]
        use libc::*;
        
        #[cfg(target_os = "windows")]
        use win_sock::*;

        let mut bound = false;

        let mut address = [0u8; 512];
        let mut address_len = 0;

        for addr in addrs.to_socket_addrs().unwrap() {
            match addr {
                net::SocketAddr::V4(v4addr) => unsafe {
                    let sin_family = &mut address as *mut u8;

                    *(sin_family as *mut u16) = AF_INET.0 as _;

                    let sin_port = sin_family.add(mem::size_of::<u16>());

                    *(sin_port as *mut u16) = v4addr.port();

                    let sin_addr = sin_port.add(mem::size_of::<u16>());

                    *(sin_addr as *mut [u8; 4]) = v4addr.ip().octets();

                    address_len = 16;
                },
                _ => todo!("ipv6 not implemented"),
            }

            if unsafe { bind(self.handle, &address as *const _ as *const _, address_len) } == -1 {
                continue;
            }

            bound = true;
            break;
        }

        if bound {
            Ok(())
        } else {
            Err(Error::AddrAlreadyInUse)
        }
    }

    pub fn send<T: Serializable>(&mut self, address: impl net::ToSocketAddrs, data: T) -> Send<T> {
        todo!()
    }
    pub fn recv<T: Deserializable>(&mut self) -> Recv<T> {
        todo!()
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        unsafe { libc::close(self.handle); }

        #[cfg(target_os = "windows")]
        unsafe { win_sock::closesocket(self.handle); }
    }
}
