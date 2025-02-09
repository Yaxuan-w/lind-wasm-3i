// use std::sync::Arc;
// use std::collections::HashMap;

use crate::fdtables;
use crate::constants::fs_constants::*;
use crate::constants::fs_constants;
use crate::sanitization::errno::*;
use crate::sanitization::syscall_conv::*;
use crate::cage::get_cage;
use libc::c_void;
use crate::rawposix::vmmap::{VmmapOps, *};
use crate::sanitization::mem_conv::*;

pub fn hello_syscall(_cageid: u64, _arg1: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    // println!("hello from cageid = {:?}", cageid);
    return 0;
}

pub fn kernel_close(fdentry: fdtables::FDTableEntry, _count: u64) {
    let _ret = unsafe {
        libc::close(fdentry.underfd as i32)
    };
}

pub fn open_syscall(cageid: u64, path_arg: u64, oflag_arg: u64, mode_arg: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    let path = convert_path_lind2host(cageid, path_arg);

    let oflag = oflag_arg as i32;
    let mode = mode_arg as u32;
    let kernel_fd = unsafe { libc::open(path.as_ptr(), oflag, mode) };

    if kernel_fd < 0 {
        return handle_errno(get_errno(), "open_syscall");
    }

    let should_cloexec = (oflag & fs_constants::O_CLOEXEC) != 0;

    match fdtables::get_unused_virtual_fd(cageid, fs_constants::FDKIND_KERNEL, kernel_fd as u64, should_cloexec, 0) {
        Ok(virtual_fd) => virtual_fd as i32,
        Err(_) => syscall_error(Errno::EMFILE, "open_syscall", "Too many files opened")
    }
}

pub fn mkdir_syscall(cageid: u64, path_arg: u64, mode_arg: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    let path = convert_path_lind2host(cageid, path_arg);

    let ret = unsafe {
        libc::mkdir(path.as_ptr(), mode_arg as u32)
    };
    if ret < 0 {
        let errno = get_errno();
        return handle_errno(errno, "mkdir");
    }
    ret
}

// syscall_handler!(write_syscall, [1 => convert_fd, 2 => convert_buf], |cageid, kernel_fd, buf, count_arg, _arg4, _arg5, _arg6| {
//     // early return
//     let count = count_arg as usize;
//     if count == 0 {
//         return 0;
//     }
//     let ret = unsafe {
//         libc::write(kernel_fd, buf, count) as i32
//     };

//     if ret < 0 {
//         let errno = get_errno();
//         return handle_errno(errno, "write");
//     }
//     return ret;
// });

pub fn write_syscall(cageid: u64, virtual_fd: u64, buf_arg: u64, count_arg: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    let kernel_fd = convert_fd(cageid, virtual_fd);
    let buf = convert_buf(cageid, buf_arg);;

    let count = count_arg as usize;
    if count == 0 {
        return 0;
    }
    let ret = unsafe {
        libc::write(kernel_fd, buf, count) as i32
    };

    if ret < 0 {
        let errno = get_errno();
        return handle_errno(errno, "write");
    }
    return ret;
}

/// Handles the `mmap_syscall`, interacting with the `vmmap` structure.
///
/// This function processes the `mmap_syscall` by updating the `vmmap` entries and performing
/// the necessary mmap operations. The handling logic is as follows:
/// 1. Restrict allowed flags to `MAP_FIXED`, `MAP_SHARED`, `MAP_PRIVATE`, and `MAP_ANONYMOUS`.
/// 2. Disallow `PROT_EXEC`; return `EINVAL` if the `prot` argument includes `PROT_EXEC`.
/// 3. If `MAP_FIXED` is not specified, query the `vmmap` structure to locate an available memory region.
///    Otherwise, use the address provided by the user.
/// 4. Invoke the actual `mmap` syscall with the `MAP_FIXED` flag to configure the memory region's protections.
/// 5. Update the corresponding `vmmap` entry.
///
/// # Arguments
/// * `cageid` - Identifier of the cage that initiated the `mmap` syscall.
/// * `addr` - Starting address of the memory region to mmap.
/// * `len` - Length of the memory region to mmap.
/// * `prot` - Memory protection flags (e.g., `PROT_READ`, `PROT_WRITE`).
/// * `flags` - Mapping flags (e.g., `MAP_SHARED`, `MAP_ANONYMOUS`).
/// * `fildes` - File descriptor associated with the mapping, if applicable.
/// * `off` - Offset within the file, if applicable.
///
/// # Returns
/// * `u32` - Result of the `mmap` operation. See "man mmap" for details
pub fn mmap_syscall(cageid: u64, addr_arg: u64, len_arg: u64, prot_arg: u64, flags_arg: u64, virtual_fd_arg: u64, off_arg: u64) -> i32 {
    let mut addr = addr_arg as *mut u8;
    let mut len = len_arg as usize;
    let mut prot = prot_arg as i32;
    let mut flags = flags_arg as i32;
    let mut fildes = virtual_fd_arg as i32;
    let mut off = off_arg as i64;
    
    let cage = get_cage(cageid).unwrap();

    let mut maxprot = PROT_READ | PROT_WRITE;

    // only these four flags are allowed
    let allowed_flags = MAP_FIXED as i32 | MAP_SHARED as i32 | MAP_PRIVATE as i32 | MAP_ANONYMOUS as i32;
    if flags & !allowed_flags > 0 {
        // truncate flag to remove flags that are not allowed
        flags &= allowed_flags;
    }

    if prot & PROT_EXEC > 0 {
        return syscall_error(Errno::EINVAL, "mmap", "PROT_EXEC is not allowed");
    }

    // check if the provided address is multiple of pages
    let rounded_addr = round_up_page(addr as u64);
    if rounded_addr != addr as u64 {
        return syscall_error(Errno::EINVAL, "mmap", "address it not aligned");
    }

    // offset should be non-negative and multiple of pages
    if off < 0 {
        return syscall_error(Errno::EINVAL, "mmap", "offset cannot be negative");
    }
    let rounded_off = round_up_page(off as u64);
    if rounded_off != off as u64 {
        return syscall_error(Errno::EINVAL, "mmap", "offset it not aligned");
    }

    // round up length to be multiple of pages
    let rounded_length = round_up_page(len as u64);

    let mut useraddr = addr as u32;
    // if MAP_FIXED is not set, then we need to find an address for the user
    if flags & MAP_FIXED as i32 == 0 {
        let mut vmmap = cage.vmmap.write();
        let result;
        
        // pick an address of appropriate size, anywhere
        if useraddr == 0 {
            result = vmmap.find_map_space(rounded_length as u32 >> PAGESHIFT, 1);
        } else {
            // use address user provided as hint to find address
            result = vmmap.find_map_space_with_hint(rounded_length as u32 >> PAGESHIFT, 1, addr as u32);
        }

        // did not find desired memory region
        if result.is_none() {
            return syscall_error(Errno::ENOMEM, "mmap", "no memory");
        }

        let space = result.unwrap();
        useraddr = (space.start() << PAGESHIFT) as u32;
    }

    flags |= MAP_FIXED as i32;

    // either MAP_PRIVATE or MAP_SHARED should be set, but not both
    if (flags & MAP_PRIVATE as i32 == 0) == (flags & MAP_SHARED as i32 == 0) {
        return syscall_error(Errno::EINVAL, "mmap", "invalid flags");
    }

    let vmmap = cage.vmmap.read();

    let sysaddr = vmmap.user_to_sys(useraddr);

    drop(vmmap);

    if rounded_length > 0 {
        if flags & MAP_ANONYMOUS as i32 > 0 {
            fildes = -1;
        }

        let result = mmap_inner(cageid, sysaddr as *mut u8, rounded_length as usize, prot, flags, fildes, off);
        
        let vmmap = cage.vmmap.read();
        let result = vmmap.sys_to_user(result);
        drop(vmmap);

        // if mmap addr is positive, that would mean the mapping is successful and we need to update the vmmap entry
        if result >= 0 {
            if result != useraddr {
                panic!("MAP_FIXED not fixed");
            }

            let mut vmmap = cage.vmmap.write();
            let backing = {
                if flags as u32 & MAP_ANONYMOUS > 0 {
                    MemoryBackingType::Anonymous
                } else {
                    // if we are doing file-backed mapping, we need to set maxprot to the file permission
                    let flags = fcntl_syscall(cageid, fildes as u64, F_GETFL as u64, 0, 0, 0, 0);
                    if flags < 0 {
                        return syscall_error(Errno::EINVAL, "mmap", "invalid file descriptor") as i32;
                    }
                    maxprot &= flags;
                    MemoryBackingType::FileDescriptor(fildes as u64)
                }
            };

            // update vmmap entry
            let _ = vmmap.add_entry_with_overwrite(useraddr >> PAGESHIFT,
                                           (rounded_length >> PAGESHIFT) as u32,
                                           prot,
                                           maxprot,
                                           flags,
                                           backing,
                                           off,
                                           len as i64,
                                           cageid);
        }
    }

    useraddr as i32
}

pub fn mmap_inner(
    cageid: u64,
    addr: *mut u8,
    len: usize,
    prot: i32,
    flags: i32,
    virtual_fd: i32,
    off: i64
) -> usize {
    if virtual_fd != -1 {
        match fdtables::translate_virtual_fd(cageid, virtual_fd as u64) {
            Ok(kernel_fd) => {
                let ret = unsafe {
                    (libc::mmap(addr as *mut c_void, len, prot, flags, kernel_fd.underfd as i32, off) as i64)
                };

                // Check if mmap failed and return the appropriate error if so
                if ret == -1 {
                    return syscall_error(Errno::EINVAL, "mmap", "mmap failed with invalid flags") as usize;
                }

                ret as usize
            },
            Err(_e) => {
                return syscall_error(Errno::EBADF, "mmap", "Bad File Descriptor") as usize;
            }
        }
    } else {
        // Handle mmap with fd = -1 (anonymous memory mapping or special case)
        let ret = unsafe {
            libc::mmap(addr as *mut c_void, len, prot, flags, -1, off) as i64
        };
        // Check if mmap failed and return the appropriate error if so
        if ret == -1 {
            return syscall_error(Errno::EINVAL, "mmap", "mmap failed with invalid flags") as usize;
        }

        ret as usize
    }
}

/// Handles the `brk_syscall`, interacting with the `vmmap` structure.
///
/// This function processes the `brk_syscall` by updating the `vmmap` entries and performing
/// the necessary operations to adjust the program break. Specifically, it updates the program 
/// break by modifying the end of the heap entry (the first entry in `vmmap`) and invokes `mmap` 
/// to adjust the memory protection as needed.
///
/// # Arguments
/// * `cageid` - Identifier of the cage that initiated the `brk` syscall.
/// * `brk` - The new program break address.
///
/// # Returns
/// * `u32` - Returns `0` on success or `-1` on failure.
/// 
pub fn brk_syscall(cageid: u64, brk_arg: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    let brk = brk_arg as i32;
    let cage = get_cage(cageid).unwrap();

    let mut vmmap = cage.vmmap.write();
    let heap = vmmap.find_page(HEAP_ENTRY_INDEX).unwrap().clone();

    assert!(heap.npages == vmmap.program_break);

    let old_brk_page = heap.npages;
    // round up the break to multiple of pages
    let brk_page = (round_up_page(brk as u64) >> PAGESHIFT) as u32;

    // if we are incrementing program break, we need to check if we have enough space
    if brk_page > old_brk_page {
        if vmmap.check_existing_mapping(old_brk_page, brk_page - old_brk_page, 0) {
            return syscall_error(Errno::ENOMEM, "brk", "no memory");
        }
    }

    // update vmmap entry
    vmmap.add_entry_with_overwrite(0, brk_page, heap.prot, heap.maxprot, heap.flags, heap.backing, heap.file_offset, heap.file_size, heap.cage_id);
    
    let old_heap_end_usr = (old_brk_page * PAGESIZE) as u32;
    let old_heap_end_sys = vmmap.user_to_sys(old_heap_end_usr)as *mut u8;

    let new_heap_end_usr = (brk_page * PAGESIZE) as u32;
    let new_heap_end_sys = vmmap.user_to_sys(new_heap_end_usr)as *mut u8;

    vmmap.set_program_break(brk_page);

    drop(vmmap);

    // if new brk is larger than old brk
    // we need to mmap the new region
    if brk_page > old_brk_page {
        let ret = mmap_inner(
            cageid,
            old_heap_end_sys,
            ((brk_page - old_brk_page) * PAGESIZE) as usize,
            heap.prot,
            (heap.flags as u32 | MAP_FIXED) as i32,
            -1,
            0
        );
        
        if ret < 0 {
            panic!("brk mmap failed");
        }
    }
    // if we are shrinking the brk
    // we need to do something similar to munmap
    // to unmap the extra memory
    else if brk_page < old_brk_page {
        let ret = mmap_inner(
            cageid,
            new_heap_end_sys,
            ((old_brk_page - brk_page) * PAGESIZE) as usize,
            PROT_NONE,
            (MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED) as i32,
            -1,
            0
        );
        
        if ret < 0 {
            panic!("brk mmap failed");
        }
    }

    0
}

/// Handles the `sbrk_syscall`, interacting with the `vmmap` structure.
///
/// This function processes the `sbrk_syscall` by updating the `vmmap` entries and managing
/// the program break. It calculates the target program break after applying the specified
/// increment and delegates further processing to the `brk_handler`.
///
/// # Arguments
/// * `cageid` - Identifier of the cage that initiated the `sbrk` syscall.
/// * `brk` - Increment to adjust the program break, which can be negative.
///
/// # Returns
/// * `u32` - Result of the `sbrk` operation. Refer to `man sbrk` for details.
pub fn sbrk_syscall(cageid: u64, sbrk_arg: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    let brk = sbrk_arg as i32;
    let cage = get_cage(cageid).unwrap();

    // get the heap entry
    let mut vmmap = cage.vmmap.read();
    let heap = vmmap.find_page(HEAP_ENTRY_INDEX).unwrap().clone();

    // program break should always be the same as the heap entry end
    assert!(heap.npages == vmmap.program_break);

    // pass 0 to sbrk will just return the current brk
    if brk == 0 {
        return (PAGESIZE * heap.npages) as i32;
    }

    // round up the break to multiple of pages
    // brk increment could possibly be negative
    let brk_page;
    if brk < 0 {
        brk_page = -((round_up_page(-brk as u64) >> PAGESHIFT) as i32);
    } else {
        brk_page = (round_up_page(brk as u64) >> PAGESHIFT) as i32;
    }

    // drop the vmmap so that brk_handler will not deadlock
    drop(vmmap);

    if brk_syscall(cageid, ((heap.npages as i32 + brk_page) << PAGESHIFT) as u64, 0, 0, 0, 0, 0) < 0 {
        return syscall_error(Errno::ENOMEM, "sbrk", "no memory") as i32;
    }

    // sbrk syscall should return previous brk address before increment
    (PAGESIZE * heap.npages) as i32
}

pub fn fcntl_syscall(cageid: u64, _arg1: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    0
}