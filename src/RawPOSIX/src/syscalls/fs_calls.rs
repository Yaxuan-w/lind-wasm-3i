//! File System Syscall Implementation
//!
//! This file provides all system related syscall implementation in RawPOSIX
use crate::arg_conversion::syscall_conv::*;
use crate::cage::get_cage;
use sysdefs::err_constants::{get_errno, handle_errno, syscall_error, Errno};
use sysdefs::fs_constants;
use sysdefs::fs_constants::*;
use fdtables;
use crate::memory::mem_helper::*;
use crate::memory::vmmap::{VmmapOps, *};
use libc::c_void;

/// Used for testing purpose
/// TODO: Remove after developing
pub fn hello_syscall(
    _cageid: u64,
    _arg1: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i32 {
    // println!("hello from cageid = {:?}", cageid);
    return 0;
}

/// Helper function for close_syscall
///
/// This function will perform kernel close when necessary
pub fn kernel_close(fdentry: fdtables::FDTableEntry, _count: u64) {
    let _ret = unsafe { libc::close(fdentry.underfd as i32) };
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/open.2.html
///
/// Linux `open()` syscall will open a file descriptor and set file status and permissions according to user needs. Since we
/// implement a file descriptor management subsystem (called `fdtables`), so we need to open a new virtual fd
/// after getting the kernel fd. `fdtables` currently only manage when a fd should be closed after open, so
/// then we need to set `O_CLOEXEC` flags according to input.
///
/// Input:
///     This call will only have one cageid indicates current cage, and three regular arguments same with Linux
///     - cageid: current cage
///     - path_arg: This argument points to a pathname naming the file. User's perspective.
///     - oflag_arg: This argument contains the file status flags and file access modes which will be alloted to
///                 the open file description. The flags are combined together using a bitwise-inclusive-OR and the
///                 result is passed as an argument to the function. We need to check if `O_CLOEXEC` has been set.
///     - mode_arg: This represents the permission of the newly created file. Directly passing to kernel.
pub fn open_syscall(
    cageid: u64,
    path_arg: u64,
    path_arg_cageid: u64,
    oflag_arg: u64,
    olag_arg_cageid: u64,
    mode_arg: u64,
    mode_arg_cageid: u64,
    _arg4: u64,
    _arg4_cageid: u64,
    _arg5: u64,
    _arg5_cageid: u64,
    _arg6: u64,
    _arg6_cageid: u64,
) -> i32 {
    // Type conversion
    let pathname = sc_convert_strncpy_from_cage(path_arg, path_arg_cageid, cageid, MAXPATH);
    let oflag = sc_convert_sysarg_to_i32(arg2, arg2_cageid, cageid);  // Note the cageid here isn't really relevant because the argument is pass-by-value.   But it could be checked to ensure it's not set to something unexpected.  
    let mode = sc_convert_sysarg_to_u32(arg3, arg3_cageid, cageid);
    sc_unusedarg(arg4, arg4_cageid, cageid); // would sometimes check, sometimes be a no-op depending on the compiler settings
    sc_unusedarg(arg5, arg5_cageid, cageid);
    sc_unusedarg(arg6, arg6_cageid, cageid);

    let path = convert_path_lind2host(cageid, path_arg);
    let oflag = oflag_arg as i32;
    let mode = mode_arg as u32;

    // Get the kernel fd first
    let kernel_fd = unsafe { libc::open(path.as_ptr(), oflag, mode) };

    if kernel_fd < 0 {
        return handle_errno(get_errno(), "open_syscall");
    }

    // Check if `O_CLOEXEC` has been est
    let should_cloexec = (oflag & fs_constants::O_CLOEXEC) != 0;

    // Mapping a new virtual fd and set `O_CLOEXEC` flag
    match fdtables::get_unused_virtual_fd(
        cageid,
        fs_constants::FDKIND_KERNEL,
        kernel_fd as u64,
        should_cloexec,
        0,
    ) {
        Ok(virtual_fd) => virtual_fd as i32,
        Err(_) => syscall_error(Errno::EMFILE, "open_syscall", "Too many files opened"),
    }
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/mkdir.2.html
///
/// Linux `mkdir()` syscall creates a new directory named by the path name pointed to by a path as the input parameter
/// in the function. Since path seen by user is different from actual path on host, we need to convert the path first.
/// RawPOSIX doesn't have any other operations, so all operations will be handled by host. RawPOSIX does error handling
/// for this syscall.
///
/// Input:
///     - cageid: current cageid
///     - path_arg: This argument points to a pathname naming the file. User's perspective.
///     - mode_arg: This represents the permission of the newly created file. Directly passing to kernel.
///
/// Return:
///     - return zero on success.  On error, -1 is returned and errno is set to indicate the error.
pub fn mkdir_syscall(
    cageid: u64,
    path_arg: u64,
    mode_arg: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i32 {
    // Type conversion
    let path = convert_path_lind2host(cageid, path_arg);
    let mode = mode_arg as u32;

    let ret = unsafe { libc::mkdir(path.as_ptr(), mode) };
    // Error handling
    if ret < 0 {
        let errno = get_errno();
        return handle_errno(errno, "mkdir");
    }
    ret
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/write.2.html
///
/// Linux `write()` syscall attempts to write `count` bytes from the buffer pointed to by `buf` to the file associated
/// with the open file descriptor, `fd`. RawPOSIX first converts virtual fd to kernel fd due to the `fdtable` subsystem, second
/// translates the `buf_arg` pointer to actual system pointer
///
/// Input:
///     - cageid: current cageid
///     - virtual_fd: virtual file descriptor, needs to be translated kernel fd for future kernel operation
///     - buf_arg: pointer points to a buffer that stores the data
///     - count_arg: length of the buffer
///
/// Output:
///     - Upon successful completion of this call, we return the number of bytes written. This number will never be greater
///         than `count`. The value returned may be less than `count` if the write_syscall() was interrupted by a signal, or
///         if the file is a pipe or FIFO or special file and has fewer than `count` bytes immediately available for writing.
pub fn write_syscall(
    cageid: u64,
    virtual_fd: u64,
    buf_arg: u64,
    count_arg: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i32 {
    let kernel_fd = convert_fd(cageid, virtual_fd);
    let buf = convert_buf(cageid, buf_arg);

    let count = count_arg as usize;
    if count == 0 {
        return 0;
    }
    let ret = unsafe { libc::write(kernel_fd, buf as *const c_void, count) as i32 };

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
pub fn mmap_syscall(
    cageid: u64,
    addr_arg: u64,
    len_arg: u64,
    prot_arg: u64,
    flags_arg: u64,
    virtual_fd_arg: u64,
    off_arg: u64,
) -> i32 {
    let mut addr = addr_arg as *mut u8;
    let mut len = len_arg as usize;
    let mut prot = prot_arg as i32;
    let mut flags = flags_arg as i32;
    let mut fildes = virtual_fd_arg as i32;
    let mut off = off_arg as i64;

    let cage = get_cage(cageid).unwrap();

    let mut maxprot = PROT_READ | PROT_WRITE;

    // only these four flags are allowed
    let allowed_flags =
        MAP_FIXED as i32 | MAP_SHARED as i32 | MAP_PRIVATE as i32 | MAP_ANONYMOUS as i32;
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
            result =
                vmmap.find_map_space_with_hint(rounded_length as u32 >> PAGESHIFT, 1, addr as u32);
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

        let result = mmap_inner(
            cageid,
            sysaddr as *mut u8,
            rounded_length as usize,
            prot,
            flags,
            fildes,
            off,
        );

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
                        return syscall_error(Errno::EINVAL, "mmap", "invalid file descriptor")
                            as i32;
                    }
                    maxprot &= flags;
                    MemoryBackingType::FileDescriptor(fildes as u64)
                }
            };

            // update vmmap entry
            let _ = vmmap.add_entry_with_overwrite(
                useraddr >> PAGESHIFT,
                (rounded_length >> PAGESHIFT) as u32,
                prot,
                maxprot,
                flags,
                backing,
                off,
                len as i64,
                cageid,
            );
        }
    }

    useraddr as i32
}

/// Helper function for `mmap` / `munmap`
///
/// This function calls underlying libc::mmap and serves as helper functions for memory related (vmmap related)
/// syscalls. This function provides fd translation between virtual to kernel and error handling.
pub fn mmap_inner(
    cageid: u64,
    addr: *mut u8,
    len: usize,
    prot: i32,
    flags: i32,
    virtual_fd: i32,
    off: i64,
) -> usize {
    if virtual_fd != -1 {
        match fdtables::translate_virtual_fd(cageid, virtual_fd as u64) {
            Ok(kernel_fd) => {
                let ret = unsafe {
                    libc::mmap(
                        addr as *mut c_void,
                        len,
                        prot,
                        flags,
                        kernel_fd.underfd as i32,
                        off,
                    ) as i64
                };

                // Check if mmap failed and return the appropriate error if so
                if ret == -1 {
                    return syscall_error(Errno::EINVAL, "mmap", "mmap failed with invalid flags")
                        as usize;
                }

                ret as usize
            }
            Err(_e) => {
                return syscall_error(Errno::EBADF, "mmap", "Bad File Descriptor") as usize;
            }
        }
    } else {
        // Handle mmap with fd = -1 (anonymous memory mapping or special case)
        let ret = unsafe { libc::mmap(addr as *mut c_void, len, prot, flags, -1, off) as i64 };
        // Check if mmap failed and return the appropriate error if so
        if ret == -1 {
            return syscall_error(Errno::EINVAL, "mmap", "mmap failed with invalid flags") as usize;
        }

        ret as usize
    }
}

/// Handler of the `munmap_syscall`, interacting with the `vmmap` structure.
///
/// This function processes the `munmap_syscall` by updating the `vmmap` entries and managing
/// the unmap operation. Instead of invoking the actual `munmap` syscall, the unmap operation
/// is simulated by setting the specified region to `PROT_NONE`. The memory remains valid but
/// becomes inaccessible due to the `PROT_NONE` setting.
///
/// # Arguments
/// * `cageid` - Identifier of the cage that calls the `munmap`
/// * `addr` - Starting address of the region to unmap
/// * `length` - Length of the region to unmap
///
/// # Returns
/// * `i32` - 0 for success and -1 for failure
pub fn munmap_syscall(
    cageid: u64,
    addr_arg: u64,
    len_arg: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i32 {
    let addr = addr_arg as *mut u8;
    let len = len_arg as usize;

    if len == 0 {
        return syscall_error(Errno::EINVAL, "munmap", "length cannot be zero");
    }
    let cage = get_cage(cageid).unwrap();

    // check if the provided address is multiple of pages
    let rounded_addr = round_up_page(addr as u64) as usize;
    if rounded_addr != addr as usize {
        return syscall_error(Errno::EINVAL, "mmap", "address it not aligned");
    }

    let vmmap = cage.vmmap.read();
    let sysaddr = vmmap.user_to_sys(rounded_addr as u32);
    drop(vmmap);

    let rounded_length = round_up_page(len as u64) as usize;

    // we are replacing munmap with mmap because we do not want to really deallocate the memory region
    // we just want to set the prot of the memory region back to PROT_NONE
    // Directly call libc::mmap to improve performance
    let result = unsafe {
        libc::mmap(
            sysaddr as *mut c_void,
            rounded_length,
            PROT_NONE,
            (MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED) as i32,
            -1,
            0,
        ) as usize
    };
    if result != sysaddr {
        panic!("MAP_FIXED not fixed");
    }

    let mut vmmap = cage.vmmap.write();

    vmmap.remove_entry(rounded_addr as u32 >> PAGESHIFT, len as u32 >> PAGESHIFT);

    0
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
pub fn brk_syscall(
    cageid: u64,
    brk_arg: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i32 {
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
    vmmap.add_entry_with_overwrite(
        0,
        brk_page,
        heap.prot,
        heap.maxprot,
        heap.flags,
        heap.backing,
        heap.file_offset,
        heap.file_size,
        heap.cage_id,
    );

    let old_heap_end_usr = (old_brk_page * PAGESIZE) as u32;
    let old_heap_end_sys = vmmap.user_to_sys(old_heap_end_usr) as *mut u8;

    let new_heap_end_usr = (brk_page * PAGESIZE) as u32;
    let new_heap_end_sys = vmmap.user_to_sys(new_heap_end_usr) as *mut u8;

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
            0,
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
            0,
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
pub fn sbrk_syscall(
    cageid: u64,
    sbrk_arg: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i32 {
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

    if brk_syscall(
        cageid,
        ((heap.npages as i32 + brk_page) << PAGESHIFT) as u64,
        0,
        0,
        0,
        0,
        0,
    ) < 0
    {
        return syscall_error(Errno::ENOMEM, "sbrk", "no memory") as i32;
    }

    // sbrk syscall should return previous brk address before increment
    (PAGESIZE * heap.npages) as i32
}

pub fn fcntl_syscall(
    cageid: u64,
    _arg1: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i32 {
    0
}
