// //! This file provides essential functions for handling and validating `u64` inputs, converting
// //! them to various system-specific data types needed in system calls.  It includes utilities
// //! for transforming raw pointers to typed structures, such as integer, buffer, and string pointers,
// //! as well as complex structures like polling, signal handling, timing, and socket-related types.
// //! Each function ensures safe and correct usage by performing null checks, boundary validations,
// //! and type casting, returning either a valid reference or an error if data is invalid. This design
// //! promotes secure, reliable access to memory and resources in a low-level systems environment.
// use sysdefs::data::fs_struct;
// use sysdefs::data::net_struct;
use sysdefs::data::fs_struct::PipeArray;
use sysdefs::constants::err_const::{syscall_error, Errno};
use crate::path_conv::LIND_ROOT;
use libc::*;
use std::ptr;
use sysdefs::*;
use std::mem::{size_of, zeroed};

/// Converts a user-space socket address into a host-compatible `sockaddr` used for syscalls.
pub fn sc_convert_host_sockaddr(arg: *mut u8, arg_cageid: u64, cageid: u64) -> (*mut sockaddr, u32) {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            return -1;
        }
    }

    let mut saddr = SockAddr::clone_to_sockaddr(arg);

    if (saddr.sun_family as i32) == AF_UNIX {
        unsafe {
            let sun_path_ptr = saddr.sun_path.as_mut_ptr();
            let path_len = strlen(sun_path_ptr);
            let lind_root_len = LIND_ROOT.len();
            let new_path_len = path_len + lind_root_len;

            if new_path_len < 108 {
                memmove(
                    sun_path_ptr.add(lind_root_len) as *mut c_void,
                    sun_path_ptr as *const c_void,
                    path_len,
                );
                memcpy(
                    sun_path_ptr as *mut c_void,
                    LIND_ROOT.as_ptr() as *const c_void,
                    lind_root_len,
                );
                memset(
                    sun_path_ptr.add(new_path_len) as *mut c_void,
                    0,
                    108 - new_path_len,
                );
            }
        }
    }
    let boxed = Box::new(saddr);
    let ptr = Box::into_raw(boxed) as *mut sockaddr_un;
    let ptr = ptr.cast::<sockaddr>();
    let len = unsafe { (*(ptr as *mut SockAddr)).get_len() };
    (ptr, len)
}

/// Copies a socket address structure from the kernel into user space based on the given address family.
pub fn sc_convert_copy_out_sockaddr(
    addr_arg: u64,    
    addr_arg1: u64,   
    family: u16,
) {
    let copyoutaddr = addr_arg as *mut u8;
    let addrlen = addr_arg1 as *mut u32;

    assert!(!copyoutaddr.is_null());
    assert!(!addrlen.is_null());

    let initaddrlen = unsafe { *addrlen };

    let (src_ptr, actual_len): (*const u8, u32) = match family as i32 {
        AF_INET => {
            let v4 = SockAddr::new_ipv4();
            (
                &v4 as *const _ as *const u8,
                size_of::<sockaddr_in>() as u32,
            )
        }
        AF_INET6 => {
            let v6 = SockAddr::new_ipv6();
            (
                &v6 as *const _ as *const u8,
                size_of::<sockaddr_in6>() as u32,
            )
        }
        AF_UNIX => {
            let un = SockAddr::new_unix();
            (
                &un as *const _ as *const u8,
                size_of::<sockaddr_un>() as u32,
            )
        }
        _ => return, 
    };

    let copy_len = initaddrlen.min(actual_len);
    unsafe {
        ptr::copy(src_ptr, copyoutaddr, copy_len as usize);
        *addrlen = actual_len.max(copy_len);
    }
}

pub fn get_pipearray<'a>(generic_argument: u64) -> Result<&'a mut PipeArray, i32> {
    let pointer = generic_argument as *mut PipeArray;
    if !pointer.is_null() {
        return Ok(unsafe { &mut *pointer });
    }
    return Err(syscall_error(
        Errno::EFAULT,
        "dispatcher",
        "input data not valid",
    ));
}
