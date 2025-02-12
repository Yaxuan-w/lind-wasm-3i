//! Top level Type Conversion API
//!
//! This file provides the top level type conversion API needed for actual syscall implementation
//! under src/syscalls/
use crate::arg_conversion::path_conv::*;
use crate::arg_conversion::type_conv::*;
use crate::cage::get_cage;
use sysdefs::err_constants::{syscall_error, Errno};
use fdtables;
use crate::memory::mem_helper::*;

/// Translate a received virtual file descriptor (`virtual_fd`) to real kernel file descriptor.
///
/// Return: underlying kernel file descriptor
pub fn convert_fd(cageid: u64, virtual_fd: u64) -> i32 {
    // Find corresponding virtual fd instance from `fdtable` subsystem
    let wrappedvfd = fdtables::translate_virtual_fd(cageid, virtual_fd);
    if wrappedvfd.is_err() {
        return syscall_error(Errno::EBADF, "write", "Bad File Descriptor");
    }
    let vfd = wrappedvfd.unwrap();
    // Actual kernel fd mapped with provided virtual fd
    vfd.underfd as i32
}

/// This function provides two operations: first, it translates path pointer address from WASM environment
/// to kernel system address; then, it adjusts the path from user's perspective to host's perspective,
/// which is adding `LIND_ROOT` before the path arguments. Considering actual syscall implementation
/// logic needs to pass string pointer to underlying rust libc, so this function will return `CString`
///
/// Input:
///     - cageid: required to do address translation for path pointer
///     - path_arg: the path pointer with address and contents from user's perspective. Address is
///                 32-bit (because of WASM feature).
///
/// Output:
///     - c_path: a `CString` variable stores the path from host's perspective
pub fn convert_path_lind2host(cageid: u64, path_arg: u64) -> CString {
    // Since we need to first translate the arguments address from
    let cage = get_cage(cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, path_arg).unwrap();
    let path = get_cstr(addr).unwrap();
    let c_path = add_lind_root(cageid, path);
    c_path
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
pub fn convert_buf(cageid: u64, buf_arg: u64) -> *const u8 {
    // Get cage reference for memory operations
    let cage = get_cage(cageid).unwrap();
    // Convert user buffer address to system address
    let buf = translate_vmmap_addr(&cage, buf_arg).unwrap() as *const u8;
    buf
}
