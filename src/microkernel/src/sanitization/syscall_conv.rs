use crate::sanitization::path_conv::*;
use crate::sanitization::type_conv::*;
use crate::sanitization::mem_conv::*;
use crate::sanitization::errno::*;
use crate::fdtables;
use crate::cage::get_cage;
use crate::threei::threei::CallFunc;

/// 
#[macro_export]
macro_rules! syscall_handler {
    ($name:ident, [$($index:literal => $op:ident),*], $handler:expr) => {
        pub fn $name(cageid: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64, arg6: u64) -> i32 {
            paste! {
                $(
                    let [<arg $index>] = $op(cageid, [<arg $index>]); 
                )*

                $handler(cageid, arg1, arg2, arg3, arg4, arg5, arg6)
            }
        }
    }
}

pub fn convert_fd(cageid: u64, virtual_fd: u64) -> i32 {
    let wrappedvfd = fdtables::translate_virtual_fd(cageid, virtual_fd);
    if wrappedvfd.is_err() {
        return syscall_error(Errno::EBADF, "write", "Bad File Descriptor");
    }

    let vfd = wrappedvfd.unwrap();
    vfd.underfd as i32
}

/// We decalre a 
/// If `.as_ptr()` is used, the pointer to the temporary variable is returned, and the pointer 
/// may be invalid when `$handler` is called. This is because `c_path` will be released after 
/// `convert_lind2host` ends, and the pointer returned by `as_ptr()` is left hanging. If the 
/// handler accesses this pointer, undefined behavior will be triggered.
pub fn convert_path_lind2host(cageid: u64, path_arg: u64) -> CString {
    let cage = get_cage(cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, path_arg).unwrap();
    let path = get_cstr(addr).unwrap();
    let c_path = add_lind_root(cageid, path);
    c_path
}

/// `*const u8` doesn't have lifetime so rust won't check its life validation
pub fn convert_buf(cageid: u64, buf_arg: u64) -> *const u8 {
    // Get cage reference for memory operations
    let cage = get_cage(cageid).unwrap();
    // Convert user buffer address to system address
    let buf = translate_vmmap_addr(&cage, buf_arg).unwrap() as *const u8;
    buf
}
