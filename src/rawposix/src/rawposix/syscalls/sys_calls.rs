use std::sync::atomic::Ordering;
use std::sync::atomic::Ordering::*;
use crate::fdtables;
// use std::sync::Arc;
// use parking_lot::RwLock;
// use std::sync::atomic::{AtomicI32, AtomicU64};
use crate::cage::*;
use crate::cage;

pub fn fork_syscall(cageid: u64, child_cageid: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
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
        gid: AtomicI32::new(
            selfcage.gid.load(Ordering::Relaxed),
        ),
        uid: AtomicI32::new(
            selfcage.uid.load(Ordering::Relaxed),
        ),
        egid: AtomicI32::new(
            selfcage.egid.load(Ordering::Relaxed),
        ),
        euid: AtomicI32::new(
            selfcage.euid.load(Ordering::Relaxed),
        ),
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

pub fn exit_syscall(cageid: u64, status_arg: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    let status = status_arg as i32;
    let _ = fdtables::remove_cage_from_fdtable(cageid);

    // Get the self cage
    let selfcage = cage::get_cage(cageid).unwrap();
    if selfcage.parent != cageid {
        let parent_cage = cage::get_cage(selfcage.parent);
        if let Some(parent) = parent_cage {
            parent.child_num.fetch_sub(1, SeqCst);
            let mut zombie_vec = parent.zombies.write();
            zombie_vec.push(cage::Zombie {cageid, exit_code: status });
        } else {
            // if parent already exited
            // BUG: we currently do not handle the situation where a parent has exited already
        }
    }

    println!("exit from cageid = {:?}", cageid);
    status
}

pub fn exec_syscall(cageid: u64, _arg1: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    0
}
