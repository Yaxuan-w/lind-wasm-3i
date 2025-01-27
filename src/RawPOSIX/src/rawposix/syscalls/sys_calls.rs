use crate::cage::*;
use std::sync::atomic::Ordering;
use crate::fdtables;

pub fn fork_syscall(cageid: u64, child_cageid: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    // Modify the fdtable manually 
    fdtables::copy_fdtable_for_cage(self.cageid, child_cageid).unwrap();

    let parent_vmmap = self.vmmap.read();
    let new_vmmap = parent_vmmap.clone();

    let cageobj = Cage {
        cageid: child_cageid,
        cwd: RwLock::new(self.cwd.read().clone()),
        parent: self.cageid,
        gid: AtomicI32::new(
            self.gid.load(Ordering::Relaxed),
        ),
        uid: AtomicI32::new(
            self.uid.load(Ordering::Relaxed),
        ),
        egid: AtomicI32::new(
            self.egid.load(Ordering::Relaxed),
        ),
        euid: AtomicI32::new(
            self.euid.load(Ordering::Relaxed),
        ),
        main_threadid: AtomicU64::new(0),
        zombies: RwLock::new(vec![]),
        child_num: AtomicU64::new(0),
        vmmap: interface::RwLock::new(new_vmmap),
    };
    
    // increment child counter for parent
    self.child_num.fetch_add(1, Ordering::SeqCst);

    let mut map = CAGE_MAP.write().unwrap();
    map.insert(child_cageid, cageobj);
    0
}

pub fn exit_syscall(cageid: u64, status_arg: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {
    let status = status_arg as i32;
    let _ = fdtables::remove_cage_from_fdtable(cageid);

    // Get the self cage
    let selfcage = get_cage(cageid).unwrap();
    if selfcage.parent != cageid {
        let parent_cage = get_cage(selfcage.parent);
        if let Some(parent) = parent_cage {
            parent.child_num.fetch_sub(1, SeqCst);
            let mut zombie_vec = parent.zombies.write();
            zombie_vec.push(Zombie {cageid: cageid, exit_code: status });
        } else {
            // if parent already exited
            // BUG: we currently do not handle the situation where a parent has exited already
        }
    }

    println!("exit from cageid = {:?}", cageid);
    status
}

pub fn exec_syscall(cageid: u64, pathname: u64, _arg2: u64, _arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> i32 {

}