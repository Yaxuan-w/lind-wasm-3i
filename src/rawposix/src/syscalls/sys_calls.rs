//! System syscalls implementation
//!
//! This module contains all system calls that are being emulated/faked in Lind.
use crate::syscalls::fs_calls::kernel_close;
use crate::typemap::syscall_conv::sc_unusedarg;
use crate::interface::{convert_signal_mask, RustDuration};
use cage::memory::mem_helper::*;
use cage::memory::vmmap::{VmmapOps, *};
use cage::{add_cage, cagetable_clear, get_cage, remove_cage, Cage, Zombie};
use fdtables;
use libc::sched_yield;
use libc::*;
use libc::{SIGKILL, SIGSTOP, SIG_BLOCK, SIG_UNBLOCK, SIG_SETMASK};
use parking_lot::RwLock;
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::atomic::Ordering::*;
use std::sync::atomic::{AtomicI32, AtomicU64};
use std::sync::Arc;
use std::ptr;
use sysdefs::constants::err_const::{get_errno, handle_errno, syscall_error, Errno};
use sysdefs::constants::fs_const::*;
use sysdefs::constants::{EXIT_SUCCESS, VERBOSE};
use sysdefs::data::fs_struct::{SigactionStruct, ITimerVal, Rlimit, SHM_METADATA};
use typemap::syscall_conv::*;

pub fn unmap_shm_mappings(cage: &Cage) {
    // Lock the reverse shared memory mapping collection.
    let rev_mappings = cage.rev_shm.lock();
    for mapping in rev_mappings.iter() {
        // We assume each mapping is a tuple where the second element is the shared memory ID.
        let shmid = mapping.1;
        match SHM_METADATA.shmtable.entry(shmid) {
            RustHashEntry::Occupied(mut entry) => {
                let segment = entry.get_mut();
                // Decrement the number of attachments.
                segment.shminfo.shm_nattch -= 1;
                // Update the detach time using the current timestamp.
                segment.shminfo.shm_dtime = timestamp() as isize;
                // Remove this cage's ID from the list of attached cages.
                segment.attached_cages.remove(&cage.cageid);
                // If the segment is marked for removal and no attachments remain, remove it.
                if segment.rmid && segment.shminfo.shm_nattch == 0 {
                    let key = segment.key;
                    entry.remove_entry();
                    SHM_METADATA.shmkeyidtable.remove(&key);
                }
            }
            RustHashEntry::Vacant(_) => {
                // In the 3i codebase, a missing shared memory entry here signals an invariant error.
                panic!("unmap_shm_mappings: Shared memory entry for shmid {} not found for cage {}", shmid, cage.cageid);
            }
        }
    }
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/fork.2.html
///
/// `fork_syscall` creates a new process (cage object). The newly created child process is an exact copy of the
/// parent process (the process that calls fork) apart from it's cage_id and the parent_id
/// In this function we separately handle copying fd tables and clone vmmap talbe and create a new Cage object
/// with this cloned tables.
pub fn fork_syscall(
    cageid: u64,
    child_arg: u64,        // Child's cage id
    child_arg_cageid: u64, // Child's cage id arguments cageid
    arg2: u64,
    arg2_cageid: u64,
    arg3: u64,
    arg3_cageid: u64,
    arg4: u64,
    arg4_cageid: u64,
    arg5: u64,
    arg5_cageid: u64,
    arg6: u64,
    arg6_cageid: u64,
) -> i32 {
    // would sometimes check, sometimes be a no-op depending on the compiler settings
    if !(sc_unusedarg(arg2, arg2_cageid)
        && sc_unusedarg(arg3, arg3_cageid)
        && sc_unusedarg(arg4, arg4_cageid)
        && sc_unusedarg(arg5, arg5_cageid)
        && sc_unusedarg(arg6, arg6_cageid))
    {
        return syscall_error(Errno::EFAULT, "fork", "Invalide Arguments");
    }

    // Modify the fdtable manually
    fdtables::copy_fdtable_for_cage(child_arg_cageid, child_arg).unwrap();

    // Get the self cage
    let selfcage = get_cage(child_arg_cageid).unwrap();

    let parent_vmmap = selfcage.vmmap.read();
    let new_vmmap = parent_vmmap.clone();

    let cageobj = Cage {
        cageid: child_arg,
        cwd: RwLock::new(selfcage.cwd.read().clone()),
        parent: child_arg_cageid,
        gid: AtomicI32::new(selfcage.gid.load(Relaxed)),
        uid: AtomicI32::new(selfcage.uid.load(Relaxed)),
        egid: AtomicI32::new(selfcage.egid.load(Relaxed)),
        euid: AtomicI32::new(selfcage.euid.load(Relaxed)),
        main_threadid: AtomicU64::new(0),
        zombies: RwLock::new(vec![]),
        child_num: AtomicU64::new(0),
        vmmap: RwLock::new(new_vmmap),
    };

    // increment child counter for parent
    selfcage.child_num.fetch_add(1, SeqCst);

    add_cage(child_arg, cageobj);
    0
}

/// Reference to Linux: https://man7.org/linux/man-pages/man3/exit.3.html
///
/// The exit function causes normal process(Cage) termination
/// The termination entails unmapping all memory references
/// Removing the cage object from the cage table, closing all open files which is removing corresponding fdtable
pub fn exit_syscall(
    cageid: u64,
    status_arg: u64,
    status_cageid: u64,
    arg2: u64,
    arg2_cageid: u64,
    arg3: u64,
    arg3_cageid: u64,
    arg4: u64,
    arg4_cageid: u64,
    arg5: u64,
    arg5_cageid: u64,
    arg6: u64,
    arg6_cageid: u64,
) -> i32 {
    let status = sc_convert_sysarg_to_i32(status_arg, status_cageid, cageid);
    // would sometimes check, sometimes be a no-op depending on the compiler settings
    if !(sc_unusedarg(arg2, arg2_cageid)
        && sc_unusedarg(arg3, arg3_cageid)
        && sc_unusedarg(arg4, arg4_cageid)
        && sc_unusedarg(arg5, arg5_cageid)
        && sc_unusedarg(arg6, arg6_cageid))
    {
        return syscall_error(Errno::EFAULT, "exit", "Invalide Arguments");
    }

    let _ = fdtables::remove_cage_from_fdtable(status_cageid);

    // Get the self cage
    let selfcage = get_cage(status_cageid).unwrap();
    if selfcage.parent != cageid {
        let parent_cage = get_cage(selfcage.parent);
        if let Some(parent) = parent_cage {
            parent.child_num.fetch_sub(1, SeqCst);
            let mut zombie_vec = parent.zombies.write();
            zombie_vec.push(Zombie {
                cageid: status_cageid,
                exit_code: status,
            });
        } else {
            // if parent already exited
            // BUG: we currently do not handle the situation where a parent has exited already
        }
    }

    status
}

/// Reference to Linux: https://man7.org/linux/man-pages/man3/exec.3.html
pub fn exec_syscall(
    cageid: u64,
    arg1: u64, 
    arg1_cageid: u64,
    arg2: u64, 
    arg2_cageid: u64,
    arg3: u64, 
    arg3_cageid: u64,
    arg4: u64, 
    arg4_cageid: u64,
    arg5: u64, 
    arg5_cageid: u64,
    arg6: u64, 
    arg6_cageid: u64,
) -> i32 {
    // validate that the unused arguments are indeed not used.
    if !(sc_unusedarg(arg1, arg1_cageid)
         && sc_unusedarg(arg2, arg2_cageid)
         && sc_unusedarg(arg3, arg3_cageid)
         && sc_unusedarg(arg4, arg4_cageid)
         && sc_unusedarg(arg5, arg5_cageid)
         && sc_unusedarg(arg6, arg6_cageid)) {
        return syscall_error(Errno::EFAULT, "exec", "Invalid Arguments");
    }

    // clear file descriptors that should not persist after exec.
    fdtables::empty_fds_for_exec(cageid);

    // retrieve the current cage (process) object.
    let cage = match get_cage(cageid) {
        Some(c) => c,
        None => return syscall_error(Errno::ECHILD, "exec", "Cage not found"),
    };

    // reset or clear resources as required for exec
    // reset cancel status
    cage.cancelstatus.store(false, Relaxed);
    // clear shared memory mappings
    cage.rev_shm.lock().clear();
    // clear thread table
    cage.thread_table.clear();
    // clear the virtual memory map
    {
        let mut vmmap = cage.vmmap.write();
        vmmap.clear();
    }
    // clear signal handlers and reset signal mask
    cage.signalhandler.clear();
    cage.sigset.store(0, Relaxed);
    // clear epoch handler and reset main thread id
    cage.epoch_handler.clear();
    {
        let mut threadid = cage.main_threadid.write();
        *threadid = 0;
    }

    // return 0 for success
    0
}

/// Reference to Linux: https://man7.org/linux/man-pages/man3/waitpid.3p.html
///
/// waitpid() will return the cageid of waited cage, or 0 when WNOHANG is set and there is no cage already exited
/// waitpid_syscall utilizes the zombie list stored in cage struct. When a cage exited, a zombie entry will be inserted
/// into the end of its parent's zombie list. Then when parent wants to wait for any of child, it could just check its
/// zombie list and retrieve the first entry from it (first in, first out).
pub fn waitpid_syscall(
    cageid: u64,
    cageid_arg: u64,
    cageid_arg_cageid: u64,
    status_arg: u64,
    status_cageid: u64,
    options_arg: u64,
    options_cageid: u64,
    arg4: u64,
    arg4_cageid: u64,
    arg5: u64,
    arg5_cageid: u64,
    arg6: u64,
    arg6_cageid: u64,
) -> i32 {
    let status = sc_convert_sysarg_to_i32_ref(status_arg, status_cageid, cageid);
    let options = sc_convert_sysarg_to_i32(options_arg, options_cageid, cageid);
    // would sometimes check, sometimes be a no-op depending on the compiler settings
    if !(sc_unusedarg(arg4, arg4_cageid)
        && sc_unusedarg(arg5, arg5_cageid)
        && sc_unusedarg(arg6, arg6_cageid))
    {
        return syscall_error(Errno::EFAULT, "waitpid", "Invalid Arguments");
    }

    // get the cage instance
    let cage = get_cage(cageid).unwrap();

    let mut zombies = cage.zombies.write();
    let child_num = cage.child_num.load(Relaxed);

    // if there is no pending zombies to wait, and there is no active child, return ECHILD
    if zombies.len() == 0 && child_num == 0 {
        return syscall_error(
            Errno::ECHILD,
            "waitpid",
            "no existing unwaited-for child processes",
        );
    }

    let mut zombie_opt: Option<Zombie> = None;

    // cageid <= 0 means wait for ANY child
    // cageid < 0 actually refers to wait for any child process whose process group ID equals -pid
    // but we do not have the concept of process group in lind, so let's just treat it as cageid == 0
    if cageid_arg <= 0 {
        loop {
            if zombies.len() == 0 && (options & libc::WNOHANG > 0) {
                // if there is no pending zombies and WNOHANG is set
                // return immediately
                return 0;
            } else if zombies.len() == 0 {
                // if there is no pending zombies and WNOHANG is not set
                // then we need to wait for children to exit
                // drop the zombies list before sleep to avoid deadlock
                drop(zombies);
                // TODO: replace busy waiting with more efficient mechanism
                unsafe {
                    sched_yield();
                }
                // after sleep, get the write access of zombies list back
                zombies = cage.zombies.write();
                continue;
            } else {
                // there are zombies avaliable
                // let's retrieve the first zombie
                zombie_opt = Some(zombies.remove(0));
                break;
            }
        }
    }
    // if cageid is specified, then we need to look up the zombie list for the id
    else {
        // first let's check if the cageid is in the zombie list
        if let Some(index) = zombies
            .iter()
            .position(|zombie| zombie.cageid == cageid_arg as u64)
        {
            // find the cage in zombie list, remove it from the list and break
            zombie_opt = Some(zombies.remove(index));
        } else {
            // if the cageid is not in the zombie list, then we know either
            // 1. the child is still running, or
            // 2. the cage has exited, but it is not the child of this cage, or
            // 3. the cage does not exist
            // we need to make sure the child is still running, and it is the child of this cage
            let child = get_cage(cageid_arg as u64);
            if let Some(child_cage) = child {
                // make sure the child's parent is correct
                if child_cage.parent != cage.cageid {
                    return syscall_error(
                        Errno::ECHILD,
                        "waitpid",
                        "waited cage is not the child of the cage",
                    );
                }
            } else {
                // cage does not exist
                return syscall_error(Errno::ECHILD, "waitpid", "cage does not exist");
            }

            // now we have verified that the cage exists and is the child of the cage
            loop {
                // the cage is not in the zombie list
                // we need to wait for the cage to actually exit

                // drop the zombies list before sleep to avoid deadlock
                drop(zombies);
                // TODO: replace busy waiting with more efficient mechanism
                unsafe {
                    sched_yield();
                }
                // after sleep, get the write access of zombies list back
                zombies = cage.zombies.write();

                // let's check if the zombie list contains the cage
                if let Some(index) = zombies
                    .iter()
                    .position(|zombie| zombie.cageid == cageid_arg as u64)
                {
                    // find the cage in zombie list, remove it from the list and break
                    zombie_opt = Some(zombies.remove(index));
                    break;
                }

                continue;
            }
        }
    }

    // reach here means we already found the desired exited child
    let zombie = zombie_opt.unwrap();
    // update the status
    *status = zombie.exit_code;
    println!("[rawposix|waitpid] cp-3");
    // return child's cageid
    zombie.cageid as i32
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/wait.2.html
///
/// See comments of waitpid_syscall
pub fn wait_syscall(
    cageid: u64,
    status_arg: u64,
    status_cageid: u64,
    arg2: u64,
    arg2_cageid: u64,
    arg3: u64,
    arg3_cageid: u64,
    arg4: u64,
    arg4_cageid: u64,
    arg5: u64,
    arg5_cageid: u64,
    arg6: u64,
    arg6_cageid: u64,
) -> i32 {
    // would sometimes check, sometimes be a no-op depending on the compiler settings
    if !(sc_unusedarg(arg2, arg2_cageid)
        && sc_unusedarg(arg3, arg3_cageid)
        && sc_unusedarg(arg4, arg4_cageid)
        && sc_unusedarg(arg5, arg5_cageid)
        && sc_unusedarg(arg6, arg6_cageid))
    {
        return syscall_error(Errno::EFAULT, "waitpid", "Invalid Arguments");
    }
    // left type conversion done inside waitpid_syscall
    waitpid_syscall(
        cageid,
        0,
        0,
        status_arg,
        status_cageid,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    )
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/getpid.2.html
///
/// Returns the unique cage identifier for the current process. In Lind,
/// the cage's id is used in place of the process id.
/// This function simply retrieves the cage from the system and returns its cageid.
pub fn getpid_syscall(
    cageid: u64,
    arg1: u64,
    arg1_cageid: u64,
    arg2: u64,
    arg2_cageid: u64,
    arg3: u64,
    arg3_cageid: u64,
    arg4: u64,
    arg4_cageid: u64,
    arg5: u64,
    arg5_cageid: u64,
    arg6: u64,
    arg6_cageid: u64,
) -> i32 {
    // would sometimes check, sometimes be a no-op depending on the compiler settings
    if !(sc_unusedarg(arg1, arg1_cageid)
        && sc_unusedarg(arg2, arg2_cageid)
        && sc_unusedarg(arg3, arg3_cageid)
        && sc_unusedarg(arg4, arg4_cageid)
        && sc_unusedarg(arg5, arg5_cageid)
        && sc_unusedarg(arg6, arg6_cageid))
    {
        return syscall_error(Errno::EFAULT, "exec", "Invalide Cage ID");
    }

    let cage = get_cage(cageid).unwrap();

    return cage.cageid as i32;
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/getppid.2.html
///
/// Returns the parent cage identifier for the current cage.
/// In Lind, the parent's cage id is stored in the cage's parent field.
pub fn getppid_syscall(
    cageid: u64,
    arg1: u64,
    arg1_cageid: u64,
    arg2: u64,
    arg2_cageid: u64,
    arg3: u64,
    arg3_cageid: u64,
    arg4: u64,
    arg4_cageid: u64,
    arg5: u64,
    arg5_cageid: u64,
    arg6: u64,
    arg6_cageid: u64,
) -> i32 {
    // would sometimes check, sometimes be a no-op depending on the compiler settings
    if !(sc_unusedarg(arg1, arg1_cageid)
        && sc_unusedarg(arg2, arg2_cageid)
        && sc_unusedarg(arg3, arg3_cageid)
        && sc_unusedarg(arg4, arg4_cageid)
        && sc_unusedarg(arg5, arg5_cageid)
        && sc_unusedarg(arg6, arg6_cageid))
    {
        return syscall_error(Errno::EFAULT, "exec", "Invalide Cage ID");
    }

    let cage = get_cage(cageid).unwrap();

    return cage.parent as i32;
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/getgid.2.html
///
/// Retrieves the group id (gid) for the current cage. If the cage's group id is uninitialized (i.e. -1),
/// then it updates it to a default group id defined in the constants and returns -1.
pub fn getgid_syscall(
    cageid: u64,
    arg1: u64, 
    arg1_cageid: u64,
    arg2: u64, 
    arg2_cageid: u64,
    arg3: u64, 
    arg3_cageid: u64,
    arg4: u64, 
    arg4_cageid: u64,
    arg5: u64, 
    arg5_cageid: u64,
    arg6: u64, 
    arg6_cageid: u64,
) -> i32 {
    // Validate that unused arguments are indeed unused.
    if !(sc_unusedarg(arg1, arg1_cageid)
         && sc_unusedarg(arg2, arg2_cageid)
         && sc_unusedarg(arg3, arg3_cageid)
         && sc_unusedarg(arg4, arg4_cageid)
         && sc_unusedarg(arg5, arg5_cageid)
         && sc_unusedarg(arg6, arg6_cageid)) {
        return syscall_error(Errno::EFAULT, "getgid", "Invalid arguments");
    }

    // Get the current cage.
    let cage = match get_cage(cageid) {
        Some(c) => c,
        None => return syscall_error(Errno::ECHILD, "getgid", "Cage not found"),
    };

    // Read the group id stored in the cage.
    let gid = cage.getgid.load(Relaxed);

    // If the group id is uninitialized (-1), update it to the default and return -1.
    if gid == -1 {
        cage.getgid.store(DEFAULT_GID as i32, Relaxed);
        return -1;
    }

    // Otherwise, return the default group id
    DEFAULT_GID as i32
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/getegid.2.html
///
/// Retrieves the effective group id (egid) for the current cage. If uninitialized (-1),
/// updates it to a default value and returns -1.
pub fn getegid_syscall(
    cageid: u64,
    arg1: u64, 
    arg1_cageid: u64,
    arg2: u64, 
    arg2_cageid: u64,
    arg3: u64, 
    arg3_cageid: u64,
    arg4: u64, 
    arg4_cageid: u64,
    arg5: u64, 
    arg5_cageid: u64,
    arg6: u64, 
    arg6_cageid: u64,
) -> i32 {
    // Validate that all extra arguments are unused.
    if !(sc_unusedarg(arg1, arg1_cageid)
         && sc_unusedarg(arg2, arg2_cageid)
         && sc_unusedarg(arg3, arg3_cageid)
         && sc_unusedarg(arg4, arg4_cageid)
         && sc_unusedarg(arg5, arg5_cageid)
         && sc_unusedarg(arg6, arg6_cageid))
    {
        return syscall_error(Errno::EFAULT, "getegid", "Invalid arguments");
    }

    // Retrieve the current cage.
    let cage = match get_cage(cageid) {
        Some(c) => c,
        None => return syscall_error(Errno::ECHILD, "getegid", "Cage not found"),
    };

    // Read the effective group id (egid) from the cage.
    let egid = cage.getegid.load(Relaxed);
    if egid == -1 {
        // If not set, update with the default and return -1.
        cage.getegid.store(DEFAULT_GID as i32, Relaxed);
        return -1;
    }

    // Otherwise, return the default effective group id.
    DEFAULT_GID as i32
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/getuid.2.html
///
/// Retrieves the user id (uid) for the current cage. If the cage’s uid is uninitialized (-1),
/// it updates it to the default user id and returns -1.
pub fn getuid_syscall(
    cageid: u64,
    arg1: u64, 
    arg1_cageid: u64,
    arg2: u64, 
    arg2_cageid: u64,
    arg3: u64, 
    arg3_cageid: u64,
    arg4: u64, 
    arg4_cageid: u64,
    arg5: u64, 
    arg5_cageid: u64,
    arg6: u64, 
    arg6_cageid: u64,
) -> i32 {
    // Validate unused arguments.
    if !(sc_unusedarg(arg1, arg1_cageid)
         && sc_unusedarg(arg2, arg2_cageid)
         && sc_unusedarg(arg3, arg3_cageid)
         && sc_unusedarg(arg4, arg4_cageid)
         && sc_unusedarg(arg5, arg5_cageid)
         && sc_unusedarg(arg6, arg6_cageid)) {
        return syscall_error(Errno::EFAULT, "getuid", "Invalid arguments");
    }

    // Retrieve the cage.
    let cage = match get_cage(cageid) {
        Some(c) => c,
        None => return syscall_error(Errno::ECHILD, "getuid", "Cage not found"),
    };

    // Read the current uid from the cage.
    let uid = cage.getuid.load(Relaxed);
    if uid == -1 {
        // If uid is uninitialized, set it to the default and return -1.
        cage.getuid.store(DEFAULT_UID as i32, Relaxed);
        return -1;
    }

    // Otherwise, return the stored uid (which is default in Lind's design).
    DEFAULT_UID as i32
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/geteuid.2.html
///
/// Retrieves the effective user id (euid) for the current cage. If uninitialized (-1),
/// it updates the euid to the default value and returns -1.
pub fn geteuid_syscall(
    cageid: u64,
    arg1: u64, 
    arg1_cageid: u64,
    arg2: u64, 
    arg2_cageid: u64,
    arg3: u64, 
    arg3_cageid: u64,
    arg4: u64, 
    arg4_cageid: u64,
    arg5: u64, 
    arg5_cageid: u64,
    arg6: u64, 
    arg6_cageid: u64,
) -> i32 {
    // Validate that each extra argument is unused.
    if !(sc_unusedarg(arg1, arg1_cageid)
         && sc_unusedarg(arg2, arg2_cageid)
         && sc_unusedarg(arg3, arg3_cageid)
         && sc_unusedarg(arg4, arg4_cageid)
         && sc_unusedarg(arg5, arg5_cageid)
         && sc_unusedarg(arg6, arg6_cageid)) {
        return syscall_error(Errno::EFAULT, "geteuid", "Invalid arguments");
    }

    // Retrieve the current cage (process) object.
    let cage = match get_cage(cageid) {
        Some(c) => c,
        None => return syscall_error(Errno::ECHILD, "geteuid", "Cage not found"),
    };

    // Load the effective user ID.
    let euid = cage.geteuid.load(Relaxed);
    if euid == -1 {
        // If uninitialized, update to the default and return -1.
        cage.geteuid.store(DEFAULT_UID as i32, Relaxed);
        return -1;
    }

    // Otherwise, return the default effective user ID.
    DEFAULT_UID as i32
}

pub fn sigaction_syscall(
    cageid: u64,
    sig_arg: u64, 
    sig_arg_cageid: u64,
    act_arg: u64, 
    act_arg_cageid: u64,
    oact_arg: u64, 
    oact_arg_cageid: u64,
    arg4: u64, 
    arg4_cageid: u64,
    arg5: u64, 
    arg5_cageid: u64,
    arg6: u64, arg6_cageid: u64,
) -> i32 {
    // Validate that the extra unused arguments are indeed unused.
    if !(sc_unusedarg(arg4, arg4_cageid)
         && sc_unusedarg(arg5, arg5_cageid)
         && sc_unusedarg(arg6, arg6_cageid))
    {
        return syscall_error(Errno::EFAULT, "sigaction", "Invalid extra arguments");
    }

    // Convert the signal argument.
    let sig = sig_arg as i32;

    // Safely convert act_arg to an optional reference.
    let act: Option<&SigactionStruct> = unsafe {
        if act_arg != 0 {
            Some(&*(act_arg as *const SigactionStruct))
        } else {
            None
        }
    };

    // Safely convert oact_arg to an optional mutable reference.
    let oact: Option<&mut SigactionStruct> = unsafe {
        if oact_arg != 0 {
            Some(&mut *(oact_arg as *mut SigactionStruct))
        } else {
            None
        }
    };

    // Retrieve the cage.
    let cage = match get_cage(cageid) {
        Some(c) => c,
        None => return syscall_error(Errno::ECHILD, "sigaction", "Cage not found"),
    };

    // If oact (old action pointer) is provided, fill it with the current action.
    if let Some(oact_ref) = oact {
        if let Some(current_act) = cage.signalhandler.get(&sig) {
            // Copy the current signal action into the provided memory.
            oact_ref.clone_from(current_act);
        } else {
            // If there is no current action, use a default.
            oact_ref.clone_from(&SigactionStruct::default());
        }
    }

    // If a new action is provided in act, update the signal handler.
    if let Some(new_act) = act {
        // Disallow modification for SIGKILL and SIGSTOP.
        if sig == SIGKILL || sig == SIGSTOP {
            return syscall_error(Errno::EINVAL, "sigaction", "Cannot modify SIGKILL or SIGSTOP");
        }
        // Insert the new signal action into the cage’s signal handler table.
        cage.signalhandler.insert(sig, new_act.clone());
    }

    0
}

pub fn kill_syscall(
    cageid: u64,       
    target_cage_arg: u64, 
    target_cage_arg_cageid: u64,
    sig_arg: u64, 
    sig_arg_cageid: u64,
    arg3: u64, 
    arg3_cageid: u64,
    arg4: u64, 
    arg4_cageid: u64,
    arg5: u64, 
    arg5_cageid: u64,
    arg6: u64, 
    arg6_cageid: u64,
) -> i32 {
    // Validate the unused arguments.
    if !(sc_unusedarg(arg3, arg3_cageid)
         && sc_unusedarg(arg4, arg4_cageid)
         && sc_unusedarg(arg5, arg5_cageid)
         && sc_unusedarg(arg6, arg6_cageid)) {
        return syscall_error(Errno::EFAULT, "kill", "Invalid extra arguments");
    }

    // Convert target cage id and signal value.
    let target_cage = target_cage_arg as i32;
    let sig = sig_arg as i32;

    // Validate the target cage id: it must not be negative and typically within a system-defined maximum.
    if target_cage < 0 {
        return syscall_error(Errno::EINVAL, "kill", "Invalid target cage id");
    }

    // Validate the signal number: for example, it should typically be in the range 1..32.
    if sig <= 0 || sig >= 32 {
        return syscall_error(Errno::EINVAL, "kill", "Invalid signal number");
    }

    // Optionally, you could verify that certain signals (e.g., SIGKILL, SIGSTOP)
    // are handled with special semantics; however, in this implementation we assume they are valid.

    // Attempt to send the signal using a helper function such as lind_send_signal.
    // This helper returns a boolean indicating whether the operation was successful.
    // The caller's cage id is not directly used to send the signal; instead, the target cage id is used.
    if !lind_send_signal(target_cage as u64, sig) {
        return syscall_error(Errno::ESRCH, "kill", "Target cage does not exist");
    }

    0
}

pub fn sigprocmask_syscall(
    cageid: u64,
    how_arg: u64, 
    how_arg_cageid: u64,
    set_arg: u64, 
    set_arg_cageid: u64,
    oldset_arg: u64, 
    oldset_arg_cageid: u64,
    arg4: u64, 
    arg4_cageid: u64,
    arg5: u64, 
    arg5_cageid: u64,
    arg6: u64, 
    arg6_cageid: u64,
) -> i32 {
    // Validate that the extra unused arguments are indeed unused.
    if !(sc_unusedarg(arg4, arg4_cageid)
         && sc_unusedarg(arg5, arg5_cageid)
         && sc_unusedarg(arg6, arg6_cageid))
    {
        return syscall_error(Errno::EFAULT, "sigprocmask", "Invalid extra arguments");
    }

    // Convert the "how" parameter to i32.
    let how = how_arg as i32;

    // Retrieve the current cage.
    let cage = match get_cage(cageid) {
        Some(c) => c,
        None => return syscall_error(Errno::ECHILD, "sigprocmask", "Cage not found"),
    };

    // If oldset is provided (nonzero), write the current sigset into it.
    if oldset_arg != 0 {
        unsafe {
            // Assume the signal mask is stored as a u64.
            *(oldset_arg as *mut u64) = cage.sigset.load(Relaxed);
        }
    }

    // If a new set is provided, update the signal mask.
    if set_arg != 0 {
        // Read the new signal mask value from the pointer.
        let new_mask = unsafe { *(set_arg as *const u64) };
        let current_mask = cage.sigset.load(Relaxed);
        // Determine the updated mask based on "how":
        let updated_mask = match how {
            SIG_BLOCK   => current_mask | new_mask,
            SIG_UNBLOCK => current_mask & !new_mask,
            SIG_SETMASK => new_mask,
            _           => return syscall_error(Errno::EINVAL, "sigprocmask", "Invalid how value"),
        };
        // Store the updated mask.
        cage.sigset.store(updated_mask, Relaxed);

        // Check if any unblocked signals are pending.
        // Assume cage.pending_signals is a collection of signal numbers.
        let pending = cage.pending_signals.read();
        if pending.iter().any(|&s| (new_mask & convert_signal_mask(s)) != 0) {
            // Trigger signal epoch if needed.
            crate::interface::signal_epoch_trigger(cageid);
        }
    }

    0
}

pub fn setitimer_syscall(
    cageid: u64,
    which_arg: u64, 
    which_arg_cageid: u64,
    new_value_arg: u64, 
    new_value_arg_cageid: u64,
    old_value_arg: u64, 
    old_value_arg_cageid: u64,
    arg4: u64, 
    arg4_cageid: u64,
    arg5: u64, 
    arg5_cageid: u64,
    arg6: u64, 
    arg6_cageid: u64,
) -> i32 {
    // Validate that extra arguments are indeed unused.
    if !(sc_unusedarg(arg4, arg4_cageid)
         && sc_unusedarg(arg5, arg5_cageid)
         && sc_unusedarg(arg6, arg6_cageid)) {
        return syscall_error(Errno::EFAULT, "setitimer", "Invalid extra arguments");
    }

    // Convert the "which" argument.
    let which = which_arg as i32;
    // For this example, we only implement for ITIMER_REAL.
    if which != ITIMER_REAL as i32 {
        return syscall_error(Errno::EINVAL, "setitimer", "Only ITIMER_REAL is supported");
    }

    // Retrieve the current cage.
    let cage = match get_cage(cageid) {
        Some(c) => c,
        None => return syscall_error(Errno::ECHILD, "setitimer", "Cage not found"),
    };

    // If an old timer value pointer is provided, fill it with the current timer values.
    if old_value_arg != 0 {
        // Assume that cage.interval_timer.get_itimer() returns (current_duration, interval_duration)
        let (curr_duration, interval_duration) = cage.interval_timer.get_itimer();
        unsafe {
            let old_ptr = old_value_arg as *mut ITimerVal;
            (*old_ptr).it_value.tv_sec = curr_duration.as_secs() as i64;
            // Convert microseconds using subsec_micros to get full microsecond precision.
            (*old_ptr).it_value.tv_usec = curr_duration.subsec_micros() as i64;
            (*old_ptr).it_interval.tv_sec = interval_duration.as_secs() as i64;
            (*old_ptr).it_interval.tv_usec = interval_duration.subsec_micros() as i64;
        }
    }

    // If a new timer value pointer is provided, update the interval timer.
    if new_value_arg != 0 {
        unsafe {
            let new_ptr = new_value_arg as *const ITimerVal;
            let new_val = &*new_ptr;
            // Convert new timer values using your interface's duration conversion.
            let curr_duration = RustDuration::new(
                new_val.it_value.tv_sec as u64,
                new_val.it_value.tv_usec as u32,
            );
            let interval_duration = RustDuration::new(
                new_val.it_interval.tv_sec as u64,
                new_val.it_interval.tv_usec as u32,
            );
            cage.interval_timer.set_itimer(curr_duration, interval_duration);
        }
    }

    0
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/getrlimit.2.html
///
/// Retrieves resource limits for the specified resource type (e.g. RLIMIT_NOFILE or RLIMIT_STACK).
/// On success, populates the provided `rlimit` structure with the current (rlim_cur) and maximum (rlim_max)
/// limits. For unsupported resource types, returns -1.
pub fn getrlimit(res_type: u64, rlimit: &mut Rlimit) -> i32 {
    match res_type {
        RLIMIT_NOFILE => {
            rlimit.rlim_cur = NOFILE_CUR;
            rlimit.rlim_max = NOFILE_MAX;
        }
        RLIMIT_STACK => {
            rlimit.rlim_cur = STACK_CUR;
            rlimit.rlim_max = STACK_MAX;
        }
        _ => return -1,
    }
    0
}

/// Reference to Linux: https://man7.org/linux/man-pages/man2/setrlimit.2.html
///
/// Sets the resource limits for a specified resource type. In this stub implementation, only RLIMIT_NOFILE
/// is checked. If the current limit is within bounds (NOFILE_CUR ≤ NOFILE_MAX) then it returns 0,
/// otherwise returns -1. For other resource types (including RLIMIT_STACK), the function is not implemented
/// and returns -1.
pub fn setrlimit(res_type: u64, _limit_value: u64) -> i32 {
    match res_type {
        RLIMIT_NOFILE => {
            if NOFILE_CUR > NOFILE_MAX {
                -1
            } else {
                0
            }
        },
        // For other resource types (including RLIMIT_STACK), return -1.
        _ => -1,
    }
}

/// Those functions are required by wasmtime to create the first cage. `verbosity` indicates whether
/// detailed error messages will be printed if set
pub fn lindrustinit(verbosity: isize) {
    let _ = VERBOSE.set(verbosity); //assigned to suppress unused result warning

    fdtables::register_close_handlers(FDKIND_KERNEL, fdtables::NULL_FUNC, kernel_close);

    let utilcage = Cage {
        cageid: 0,
        cwd: RwLock::new(Arc::new(PathBuf::from("/"))),
        parent: 0,
        gid: AtomicI32::new(-1),
        uid: AtomicI32::new(-1),
        egid: AtomicI32::new(-1),
        euid: AtomicI32::new(-1),
        main_threadid: AtomicU64::new(0),
        zombies: RwLock::new(vec![]),
        child_num: AtomicU64::new(0),
        vmmap: RwLock::new(Vmmap::new()),
    };

    add_cage(
        0, // cageid
        utilcage,
    );
    fdtables::init_empty_cage(0);
    // Set the first 3 fd to STDIN / STDOUT / STDERR
    // TODO:
    // Replace the hardcoded values with variables (possibly by adding a LIND-specific constants file)
    let dev_null = CString::new("/home/lind-wasm/src/RawPOSIX/tmp/dev/null").unwrap();

    // Make sure that the standard file descriptor (stdin, stdout, stderr) is always valid, even if they
    // are closed before.
    // Standard input (fd = 0) is redirected to /dev/null
    // Standard output (fd = 1) is redirected to /dev/null
    // Standard error (fd = 2) is set to copy of stdout
    unsafe {
        libc::open(dev_null.as_ptr(), libc::O_RDONLY);
        libc::open(dev_null.as_ptr(), libc::O_WRONLY);
        libc::dup(1);
    }

    // STDIN
    fdtables::get_specific_virtual_fd(
        0,
        STDIN_FILENO as u64,
        FDKIND_KERNEL,
        STDIN_FILENO as u64,
        false,
        0,
    )
    .unwrap();
    // STDOUT
    fdtables::get_specific_virtual_fd(
        0,
        STDOUT_FILENO as u64,
        FDKIND_KERNEL,
        STDOUT_FILENO as u64,
        false,
        0,
    )
    .unwrap();
    // STDERR
    fdtables::get_specific_virtual_fd(
        0,
        STDERR_FILENO as u64,
        FDKIND_KERNEL,
        STDERR_FILENO as u64,
        false,
        0,
    )
    .unwrap();

    //init cage is its own parent
    let initcage = Cage {
        cageid: 1,
        cwd: RwLock::new(Arc::new(PathBuf::from("/"))),
        parent: 1,
        gid: AtomicI32::new(-1),
        uid: AtomicI32::new(-1),
        egid: AtomicI32::new(-1),
        euid: AtomicI32::new(-1),
        main_threadid: AtomicU64::new(0),
        zombies: RwLock::new(vec![]),
        child_num: AtomicU64::new(0),
        vmmap: RwLock::new(Vmmap::new()),
    };

    // Add cage to cagetable
    add_cage(
        1, // cageid
        initcage,
    );

    fdtables::init_empty_cage(1);
    // Set the first 3 fd to STDIN / STDOUT / STDERR
    // STDIN
    fdtables::get_specific_virtual_fd(
        1,
        STDIN_FILENO as u64,
        FDKIND_KERNEL,
        STDIN_FILENO as u64,
        false,
        0,
    )
    .unwrap();
    // STDOUT
    fdtables::get_specific_virtual_fd(
        1,
        STDOUT_FILENO as u64,
        FDKIND_KERNEL,
        STDOUT_FILENO as u64,
        false,
        0,
    )
    .unwrap();
    // STDERR
    fdtables::get_specific_virtual_fd(
        1,
        STDERR_FILENO as u64,
        FDKIND_KERNEL,
        STDERR_FILENO as u64,
        false,
        0,
    )
    .unwrap();
}

pub fn lindrustfinalize() {
    let exitvec = cagetable_clear();

    for cageid in exitvec {
        exit_syscall(
            cageid as u64,       // target cageid
            EXIT_SUCCESS as u64, // status arg
            cageid as u64,       // status arg's cageid
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        );
    }
}
