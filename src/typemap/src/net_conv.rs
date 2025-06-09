use cage::{get_cage, translate_vmmap_addr};
pub use libc::*;
pub use std::time::Duration;
use sysdefs::*;
use sysdefs::constants::err_const::{get_errno, handle_errno, syscall_error, Errno};

/// Checks whether a user-space argument is null.
pub fn sc_convert_arg_nullity(arg: u64, arg_cageid: u64, cageid: u64) -> bool {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            return -1;
        }
    }
    
    (arg as *const u8).is_null()
}

/// Converts a user-space pointer into a mutable slice of `PollStruct`.
pub fn sc_convert_pollstruct_slice<'a>(
    arg: u64,
    arg_cageid: u64, 
    cageid: u64,
    nfds: usize
) -> Result<&'a mut [PollStruct], i32> {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            return -1;
        }
    }

    let pollstructptr = arg as *mut PollStruct;
    if !pollstructptr.is_null() {
        return Ok(unsafe { std::slice::from_raw_parts_mut(pollstructptr, nfds) });
    }
    return Err(syscall_error(
        Errno::EFAULT,
        "dispatcher",
        "input data not valid",
    ));
}

/// Converts a user-space pointer into a mutable reference to `EpollEvent`.
pub fn sc_convert_epollevent<'a>(arg: u64, arg_cageid: u64, cageid: u64) -> Result<&'a mut EpollEvent, i32> {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            return -1;
        }
    }

    let cage = get_cage(arg_cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, arg).unwrap();
    let epolleventptr = addr as *mut EpollEvent;
    if !epolleventptr.is_null() {
        return Ok(unsafe { &mut *epolleventptr });
    }
    return Err(syscall_error(
        Errno::EFAULT,
        "dispatcher",
        "input data not valid",
    ));
}

/// Converts a user-space pointer into a mutable slice of `EpollEvent`.
pub fn sc_convert_epollevent_slice<'a>(
    arg: u64,
    arg_cageid: u64, 
    cageid: u64,
    nfds: i32,
) -> Result<&'a mut [EpollEvent], i32> {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            return -1;
        }
    }
    let cage = get_cage(arg_cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, arg).unwrap();
    let epolleventptr = addr as *mut EpollEvent;

    if !epolleventptr.is_null() {
        return Ok(unsafe { std::slice::from_raw_parts_mut(epolleventptr, nfds as usize) });
    }

    return Err(syscall_error(
        Errno::EFAULT,
        "dispatcher",
        "input data not valid",
    ));
}

/// Converts a user-space pointer into a mutable reference to `SockPair`.
pub fn sc_convert_sockpair<'a>(arg: u64, arg_cageid: u64, cageid: u64,) -> Result<&'a mut SockPair, i32> {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            return -1;
        }
    }

    let cage = get_cage(arg_cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, arg).unwrap();
    let pointer = addr as *mut SockPair;
    if !pointer.is_null() {
        return Ok(unsafe { &mut *pointer });
    }
    return Err(syscall_error(
        Errno::EFAULT,
        "dispatcher",
        "input data not valid",
    ));
}

pub fn fill(bufptr: *mut u8, count: usize, values: &Vec<u8>) -> i32 {
    let slice = unsafe { std::slice::from_raw_parts_mut(bufptr, count) };
    slice.copy_from_slice(&values[..count]);
    count as i32
}

/// Converts a user-space pointer into an optional mutable reference to `fd_set`.
pub fn sc_convert_fdset(arg: u64, arg_cageid: u64, cageid: u64) -> Result<Option<&'static mut fd_set>, i32> {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            return -1;
        }
    }

    let data = arg as *mut libc::fd_set;
    if !data.is_null() {
        let internal_fds = unsafe { &mut *(data as *mut fd_set) };
        return Ok(Some(internal_fds));
    }
    return Ok(None);
}

/// Converts a user-space `timeval` pointer into an optional `Duration`.
pub fn sc_convert_duration_fromtimeval(arg: u64, arg_cageid: u64, cageid: u64) -> Result<Option<Duration>, i32> {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            return -1;
        }
    }

    let pointer = arg as *mut timeval;
    if !pointer.is_null() {
        let times = unsafe { &mut *pointer };
        return Ok(Some(Duration::new(
            times.tv_sec as u64,
            times.tv_usec as u32 * 1000,
        )));
    } else {
        return Ok(None);
    }
}

