use libc::{sa_family_t, sockaddr_un, sockaddr_in, sockaddr_in6, AF_UNIX, AF_INET, AF_INET6};
use libc::sockaddr;
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::os::raw::c_char;

// // create a sockaddr_un struct
// pub fn create_sockaddr_un() -> sockaddr_un{
//     sockaddr_un {
//         sun_family: 0,            
//         sun_path: [0; 108],     
//     }
// }
#[repr(C)]
pub struct SockAddr {
    pub sun_family: u16,
    pub sun_path: [c_char; 108],
}

impl SockAddr {
    pub fn new_unix() -> Self {
        SockAddr {
            sun_family: AF_UNIX as u16,
            sun_path: [0; 108],
        }
    }

    pub fn new_ipv4() -> Self {
        SockAddr {
            sun_family: AF_INET as u16,
            sun_path: [0; 108],
        }
    }

    pub fn new_ipv6() -> Self {
        SockAddr {
            sun_family: AF_INET6 as u16,
            sun_path: [0; 108],
        }
    }

    pub fn get_len(&self) -> u32 {
        match self.sun_family as i32 {
            AF_INET => mem::size_of::<libc::sockaddr_in>() as u32,
            AF_INET6 => mem::size_of::<libc::sockaddr_in6>() as u32,
            AF_UNIX => mem::size_of::<libc::sockaddr_un>() as u32,
            _ => 0,
        }
    }

    pub fn clone_to_sockaddr(addr: *mut u8) -> Self {
        let mut out = SockAddr {
            sun_family: 0,
            sun_path: [0; 108],
        };

        if addr.is_null() {
            return out;
        }

        unsafe {
            let addr = addr as *const sockaddr;
            let family = (*addr).sa_family;
            out.sun_family = family;

            let copy_len = match family as i32 {
                AF_UNIX => size_of::<sockaddr_un>() - size_of::<sa_family_t>(),
                AF_INET => size_of::<sockaddr_in>() - size_of::<sa_family_t>(),
                AF_INET6 => size_of::<sockaddr_in6>() - size_of::<sa_family_t>(),
                _ => 0,
            };

            let safe_len = std::cmp::min(copy_len, 108);

            ptr::copy_nonoverlapping(
                (addr as *const u8).add(size_of::<sa_family_t>()),
                out.sun_path.as_mut_ptr() as *mut u8,
                safe_len,
            );
        }

        out
    }
}

#[repr(C)]
pub struct PollStruct {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

#[derive(Debug)]
#[repr(C)]
pub struct EpollEvent {
    pub events: u32,
    pub fd: i32, //in native this is a union which could be one of a number of things
                 //however, we only support EPOLL_CTL subcommands which take the fd
}

#[repr(C)]
pub struct SockPair {
    pub sock1: i32,
    pub sock2: i32,
}

// Call different functions according to different needs in rawposix. But those calls should be 
// placed into net_conv.rs
// net_conv.rs:
// pub fn sc_convert_rawposix_sockaddr(arg: u64, arg_cageid: u64, cageid: u64) -> SockAddr {
    
    
    
//     // transfer addr
//     // create new SockAddr::new()
//     // memcpy contents in addr to the one created by SockAddr::new()
//     // return SockAddr
// }

