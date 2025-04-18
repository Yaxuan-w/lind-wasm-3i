//! Top level Type Conversion API
//!
//! This file provides the top level type conversion API needed for actual syscall implementation
//! under src/syscalls/
use crate::path_conv::*;
use crate::type_conv::*;
use cage::get_cage;
use cage::memory::mem_helper::*;
use fdtables;
use std::error::Error;
use std::str::Utf8Error;
use sysdefs::constants::err_const::{syscall_error, Errno};
use sysdefs::constants::fs_const::{MAX_CAGEID, PATH_MAX};

/// Translate a received virtual file descriptor (`virtual_fd`) to real kernel file descriptor.
/// This function is not for security purpose. Always using arg_cageid to translate.
///     - If arg_cageid != cageid: this call is sent by grate. We need to translate according to cage
///     - If arg_cageid == cageid: this call is sent by cage, we can use either one
/// Return: underlying kernel file descriptor
pub fn convert_fd_to_host(virtual_fd: u64, arg_cageid: u64, cageid: u64) -> i32 {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(path_arg_cageid, cageid) {
            return -1;
        }
    }
    // Find corresponding virtual fd instance from `fdtable` subsystem
    let wrappedvfd = fdtables::translate_virtual_fd(arg_cageid, virtual_fd);
    if wrappedvfd.is_err() {
        return -9;
    }
    let vfd = wrappedvfd.unwrap();
    // Actual kernel fd mapped with provided virtual fd
    vfd.underfd as i32
}

/// This function provides two operations: first, it translates path pointer address from WASM environment
/// to kernel system address; then, it adjusts the path from user's perspective to host's perspective,
/// which is adding `LIND_ROOT` before the path arguments. Considering actual syscall implementation
/// logic needs to pass string pointer to underlying rust libc, so this function will return `CString`
/// lways using arg_cageid to translate.
///     - If arg_cageid != cageid: this call is sent by grate. We need to translate according to cage
///     - If arg_cageid == cageid: this call is sent by cage, we can use either one
/// Input:
///     - cageid: required to do address translation for path pointer
///     - path_arg: the path pointer with address and contents from user's perspective. Address is
///                 32-bit (because of WASM feature).
///
/// Output:
///     - c_path: a `CString` variable stores the path from host's perspective
///     - will return error if total length exceed the MAX_PATH (which is 4096). We use `Box<dyn Error>` here to
///      let upper functions do error handling. (ie: we want to )
pub fn sc_convert_path_to_host(path_arg: u64, path_arg_cageid: u64, cageid: u64) -> CString {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(path_arg_cageid, cageid) {
            panic!("Invalide Cage ID");
        }
    }
    let cage = get_cage(path_arg_cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, path_arg).unwrap();
    let path = match get_cstr(addr) {
        Ok(path) => path,
        Err(e) => panic!("{:?}", e),
    };
    // We will create a new variable in host process to handle the path value
    let relpath = normpath(convpath(path), path_arg_cageid);
    let relative_path = relpath.to_str().unwrap();

    #[cfg(feature = "secure")]
    {
        let total_length = LIND_ROOT.len() + relative_path.len();

        if total_length >= PATH_MAX {
            panic!("Path exceeds PATH_MAX (4096)");
        }
    }

    // CString will handle the case when string is not terminated by `\0`, but will return error if `\0` is
    // contained within the string.
    let full_path = format!("{}{}", LIND_ROOT, relative_path);
    match CString::new(full_path) {
        Ok(c_path) => c_path,
        Err(_) => panic!("String contains internal null byte"),
    }
}

/// This function translates a memory address from the WASM environment (user space)
/// to the corresponding host system address (kernel space). It is typically used when
/// the guest application passes a pointer argument to a syscall, and we need to dereference
/// it in the kernel context.
/// 
/// Input:
///     - addr_arg: the raw 64-bit address from the user
///     - addr_arg_cageid: the cage ID where the address belongs to
///     - cageid: the current running cage's ID (used for checking context)
/// 
/// Output:
///     - Returns a mutable pointer to host memory corresponding to the given address
///       from the guest. The pointer can be used for direct read/write operations.
pub fn sc_convert_addr_to_host(addr_arg: u64, addr_arg_cageid: u64, cageid: u64) -> *mut u8 {
    let cage = get_cage(addr_arg_cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, addr_arg).unwrap() as *mut u8;
    return addr;
}

/// This function translates a buffer pointer from the WASM environment to a host pointer. 
/// It is typically used when a syscall needs to read a buffer (e.g., in `read`, `write`, etc).
///
/// Input:
///     - buf_arg: the raw address of the buffer in WASM space
///     - buf_arg_cageid: the cage ID of the buffer address
///     - cageid: current running cage ID
///
/// Output:
///     - Returns a constant (read-only) host pointer to the translated buffer.
///       Suitable for syscalls that only read from the buffer.
pub fn sc_convert_buf_to_host(buf_arg: u64, buf_arg_cageid: u64, cageid: u64) -> *const u8 {
    let cage = get_cage(buf_arg_cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, buf_arg).unwrap() as *mut u8;
    return addr;
}

/// This function translates 64 bits uadd from the WASM context
/// into the corresponding host address value. Unlike the previous two functions, it returns
/// the translated address as a raw `u64` rather than a pointer.
///
/// Input:
///     - uaddr_arg: the original 64-bit address from the WASM space
///     - uaddr_arg_cageid: the cage ID that owns the address
///     - cageid: the currently executing cage ID
///
/// Output:
///     - Returns the translated 64-bit address in host space as a u64.
pub fn sc_convert_uaddr_to_host(uaddr_arg: u64, uaddr_arg_cageid: u64, cageid: u64) -> u64{
    let cage = get_cage(uaddr_arg_cageid).unwrap();
    let uaddr = translate_vmmap_addr(&cage, uaddr_arg).unwrap();
    return uaddr;
}

pub unsafe fn charstar_to_ruststr<'a>(cstr: *const i8) -> Result<&'a str, Utf8Error> {
    std::ffi::CStr::from_ptr(cstr as *const _).to_str() //returns a result to be unwrapped later
}

pub fn get_cstr<'a>(arg: u64) -> Result<&'a str, i32> {
    let ptr = arg as *const i8;
    if !ptr.is_null() {
        if let Ok(data) = unsafe { charstar_to_ruststr(ptr) } {
            return Ok(data);
        } else {
            return Err(-1);
        }
    }

    return Err(-1);
}

/// This function will be called only in secure mode
pub fn validate_cageid(cageid_1: u64, cageid_2: u64) -> bool {
    if cageid_1 > MAX_CAGEID as u64 || cageid_2 > MAX_CAGEID as u64 {
        return false;
    }
    true
}

/// This function will be called only in secure mode
pub fn get_i32(arg: u64, arg_cageid: u64, cageid: u64) -> i32 {
    if !validate_cageid(arg_cageid, cageid) {
        panic!("Invalide Cage ID");
    }

    if (arg & 0xFFFFFFFF_00000000) != 1 {
        return (arg & 0xFFFFFFFF) as i32;
    }

    panic!("Invalide argument");
}

/// This function will be called only in secure mode TODO: delete
// pub fn get_i32_ref<'a>(arg: u64, arg_cageid: u64, cageid: u64) -> &'a mut i32 {
//     if !validate_cageid(arg_cageid, cageid) {
//         panic!("Invalide Cage ID");
//     }

//     unsafe { &mut *((arg) as *mut i32)}
// }

/// This function will be called only in secure mode
pub fn get_u32(arg: u64, arg_cageid: u64, cageid: u64) -> u32 {
    if !validate_cageid(arg_cageid, cageid) {
        panic!("Invalide Cage ID");
    }

    if (arg & 0xFFFFFFFF_00000000) != 1 {
        return (arg & 0xFFFFFFFF) as u32;
    }

    panic!("Invalide argument");
}

pub fn sc_convert_sysarg_to_i32(arg: u64, arg_cageid: u64, cageid: u64) -> i32 {
    #[cfg(feature = "fast")]
    return arg as i32;

    #[cfg(feature = "secure")]
    return get_i32(arg, arg_cageid, cageid);
}

pub fn sc_convert_sysarg_to_i32_ref<'a>(arg: u64, arg_cageid: u64, cageid: u64) -> &'a mut i32 {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            panic!("Invalide Cage ID");
        }
    }

    let cage = get_cage(arg_cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, arg).unwrap();
    return unsafe { &mut *((addr) as *mut i32) };
}

pub fn sc_convert_sysarg_to_u32(arg: u64, arg_cageid: u64, cageid: u64) -> u32 {
    #[cfg(feature = "fast")]
    return arg as u32;

    #[cfg(feature = "secure")]
    return get_u32(arg, arg_cageid, cageid);
}

/// If the compilation flag has been set to `secure`, then extra check
/// will be performed
pub fn sc_convert_sysarg_to_isize(arg: u64, arg_cageid: u64, cageid: u64) -> isize {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            panic!("Invalide Cage ID");
        }
    }

    return arg as isize;
}

pub fn sc_convert_sysarg_to_usize(arg: u64, arg_cageid: u64, cageid: u64) -> usize {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            panic!("Invalide Cage ID");
        }
    }

    return arg as usize;
}

pub fn sc_convert_sysarg_to_i64(arg: u64, arg_cageid: u64, cageid: u64) -> i64 {
    #[cfg(feature = "secure")]
    {
        if !validate_cageid(arg_cageid, cageid) {
            panic!("Invalide Cage ID");
        }
    }

    return arg as i64;
}

pub fn sc_unusedarg(arg: u64, arg_cageid: u64) -> bool {
    #[cfg(feature = "fast")]
    return true;

    #[cfg(feature = "secure")]
    return !(arg | arg_cageid);
}

/// This function translates the buffer pointer from user buffer address to system address, because we are
/// transferring between 32-bit WASM environment to 64-bit kernel
///
/// Input:
///     - cageid: required to do address translation for buf pointer
///     - buf_arg: the buf pointer address, which is 32-bit because of WASM feature
///
/// Output:
///     - buf: actual system address, which is the actual position that stores data
pub fn sc_convert_buf(buf_arg: u64, arg_cageid: u64, cageid: u64) -> *const u8 {
    // Get cage reference to translate address
    let cage = get_cage(arg_cageid).unwrap();
    // Convert user buffer address to system address. We don't need to check permission here.
    // Permission check has been handled in 3i
    let buf = translate_vmmap_addr(&cage, buf_arg).unwrap() as *const u8;
    buf
}
