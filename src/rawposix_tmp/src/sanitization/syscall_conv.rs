use crate::sanitization::path_conv::*;
use crate::sanitization::type_conv::*;
use crate::sanitization::mem_conv::*;
use crate::sanitization::errno::*;
use crate::fdtables;
use crate::cage::get_cage;
use crate::threei::threei::CallFunc;

/// compiler needs to know variable type ==> match first then assign
/// different index corresponding to different op ==> use different match
/// all op return type should be same ==> use different match
// #[macro_export]
// macro_rules! syscall_handler {
//     ($name:ident, [$($index:literal => $op:ident),*], $handler:expr) => {
//         pub fn $name(cageid: u64, mut arg1: u64, mut arg2: u64, mut arg3: u64, mut arg4: u64, mut arg5: u64, mut arg6: u64) -> i32 {
//             $(
//                 let conv_arg1 = match $index {
//                     1 => $op(cageid, arg1),
//                     _ => arg1,
//                 };
//                 let conv_arg2 = match $index {
//                     2 => $op(cageid, arg2),
//                     _ => arg2,
//                 };
//                 let conv_arg3 = match $index {
//                     3 => $op(cageid, arg3),
//                     _ => arg3,
//                 };
//                 let conv_arg4 = match $index {
//                     4 => $op(cageid, arg4),
//                     _ => arg4,
//                 };
//                 let conv_arg5 = match $index {
//                     5 => $op(cageid, arg5),
//                     _ => arg5,
//                 };
//                 let conv_arg6 = match $index {
//                     6 => $op(cageid, arg6),
//                     _ => arg6,
//                 };
//             )*

//             $handler(cageid, conv_arg1, conv_arg2, conv_arg3, conv_arg4, conv_arg5, conv_arg6)
//         }
//     }
// }

pub fn convert_fd(cageid: u64, virtual_fd: u64) -> i32 {
    let wrappedvfd = fdtables::translate_virtual_fd(cageid, virtual_fd);
    if wrappedvfd.is_err() {
        return syscall_error(Errno::EBADF, "write", "Bad File Descriptor");
    }

    let vfd = wrappedvfd.unwrap();
    vfd.underfd as i32
}

pub fn convert_path_lind2host(cageid: u64, path_arg: u64) -> CString {
    let cage = get_cage(cageid).unwrap();
    let addr = translate_vmmap_addr(&cage, path_arg).unwrap();
    let path = get_cstr(addr).unwrap();
    let c_path = add_lind_root(cageid, path);
    c_path
}

pub fn convert_buf(cageid: u64, buf_arg: u64) -> *const u8 {
    // Get cage reference for memory operations
    let cage = get_cage(cageid).unwrap();
    // Convert user buffer address to system address
    let buf = translate_vmmap_addr(&cage, buf_arg).unwrap() as *const u8;
    buf
}
