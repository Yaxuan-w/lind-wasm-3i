//! System syscalls implementation
//!
//! This module contains all system calls that are being emulated/faked in Lind.
use crate::cage;
use crate::cage::*;
use crate::fdtables;
use std::sync::atomic::Ordering;
use std::sync::atomic::Ordering::*;

/// Reference to Linux: https://man7.org/linux/man-pages/man2/fork.2.html
///
/// `fork_syscall` creates a new process (cage object). The newly created child process is an exact copy of the
/// parent process (the process that calls fork) apart from it's cage_id and the parent_id
/// In this function we separately handle copying fd tables and clone vmmap talbe and create a new Cage object
/// with this cloned tables.
pub fn fork_syscall(
    cageid: u64,
    child_cageid: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i32 {
    // Modify the fdtable manually
    fdtables::copy_fdtable_for_cage(cageid, child_cageid).unwrap();

    // Get the self cage
    let selfcage = cage::get_cage(cageid).unwrap();

    let parent_vmmap = selfcage.vmmap.read();
    let new_vmmap = parent_vmmap.clone();

    let cageobj = cage::Cage {
        cageid: child_cageid,
        cwd: RwLock::new(selfcage.cwd.read().clone()),
        parent: cageid,
        gid: AtomicI32::new(selfcage.gid.load(Ordering::Relaxed)),
        uid: AtomicI32::new(selfcage.uid.load(Ordering::Relaxed)),
        egid: AtomicI32::new(selfcage.egid.load(Ordering::Relaxed)),
        euid: AtomicI32::new(selfcage.euid.load(Ordering::Relaxed)),
        main_threadid: AtomicU64::new(0),
        zombies: RwLock::new(vec![]),
        child_num: AtomicU64::new(0),
        vmmap: RwLock::new(new_vmmap),
    };

    // increment child counter for parent
    selfcage.child_num.fetch_add(1, Ordering::SeqCst);

    add_cage(child_cageid, cageobj);
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
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> i32 {
    let status = status_arg as i32;
    let _ = fdtables::remove_cage_from_fdtable(cageid);

    // Get the self cage
    let selfcage = cage::get_cage(cageid).unwrap();
    if selfcage.parent != cageid {
        let parent_cage = cage::get_cage(selfcage.parent);
        if let Some(parent) = parent_cage {
            parent.child_num.fetch_sub(1, SeqCst);
            let mut zombie_vec = parent.zombies.write();
            zombie_vec.push(cage::Zombie {
                cageid,
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
///
/// In our implementation, WASM is responsible for handling functionalities such as loading and executing
/// the new program, preserving process attributes, and resetting memory and the stack.
///
/// In RawPOSIX, the focus is on memory management inheritance and resource cleanup and release. Specifically,
/// RawPOSIX handles tasks such as clearing memory mappings, resetting shared memory, managing file descriptors
/// (closing or inheriting them based on the `should_cloexec` flag in fdtable), resetting semaphores, and
/// managing process attributes and threads (terminating unnecessary threads). This allows us to fully implement
/// the exec functionality while aligning with POSIX standards. Cage fields remained in exec():
/// cageid, cwd, parent, interval_timer
pub fn exec_syscall(
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
