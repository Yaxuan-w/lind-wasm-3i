//! This file contains all the implementation related to Cage structure. Including structure 
//! definitions, a global variables that handles cage management, and cage initialization and
//! finialization required by wasmtime
pub use std::collections::HashMap;
pub use std::sync::Arc;
/// Uses spinlocks first (for short waits) and parks threads when blocking to reduce kernel 
/// interaction and increases efficiency.
pub use parking_lot::RwLock;
pub use once_cell::sync::Lazy;
pub use std::sync::atomic::{AtomicI32, AtomicU64};
pub use std::path::{Path, PathBuf};
use crate::rawposix::vmmap::*;
use crate::rawposix::syscalls::sys_calls::exit_syscall;
use crate::rawposix::syscalls::fs_calls::kernel_close;
use crate::fdtables;
use std::ffi::CString;
use crate::constants::{fs_constants, sys_constants};
use crate::sanitization::errno::VERBOSE;

#[derive(Debug, Clone, Copy)]
pub struct Zombie {
    pub cageid: u64,
    pub exit_code: i32
}

/// I only kept required fields for cage struct
#[derive(Debug)]
pub struct Cage {
    // Identifying ID number for this cage
    pub cageid: u64,
    pub parent: u64,
    // Current working directory of cage, must be able to be unique from other cages
    pub cwd: RwLock<Arc<PathBuf>>, 
    // Identifiers for gid/uid/egid/euid 
    pub gid: AtomicI32,
    pub uid: AtomicI32,
    pub egid: AtomicI32,
    pub euid: AtomicI32,
    // The kernel thread id of the main thread of current cage, used because when we want to send signals, 
    // we want to send to the main thread 
    pub main_threadid: AtomicU64,
    // The zombies field in the Cage struct is used to manage information about child cages that have 
    // exited, but whose exit status has not yet been retrieved by their parent using wait() / waitpid().
    // When a cage exits, shared memory segments are detached, file descriptors are removed from fdtable, 
    // and cage struct is cleaned up, but its exit status are inserted along with its cage id into the end of 
    // its parent cage's zombies list
    pub zombies: RwLock<Vec<Zombie>>,
    pub child_num: AtomicU64,
    pub vmmap: RwLock<Vmmap>
}

/// We achieve an O(1) complexity for our cage map implementation through the following three approaches:
/// 
/// Direct Indexing with `cageid`:
///     `cageid` directly as the index to access the `Vec`, allowing O(1) complexity for lookup, insertion, 
///     and deletion.
/// `Vec<Option<Arc<Cage>>>` for Efficient Deletion:
///     When deleting an entry, we replace it with `None` instead of restructuring the `Vec`. If we were to 
///     use `Vec<Arc<Cage>>`, there would be no empty slots after deletion, forcing us to use `retain()` to 
///     reallocate the `Vec`, which results in O(n) complexity. Using `Vec<Option<Arc<Cage>>>` allows us to 
///     maintain O(1) deletion complexity.
/// `RwLock` for Concurrent Access Control:
///     `RwLock` ensures thread-safe access to `CAGE_MAP`, providing control over concurrent reads and writes. 
///     Since writes occur only during initialization (`lindrustinit`) and `fork()` / `exec()`, and deletions 
///     happen only via `exit()`, the additional overhead introduced by `RwLock` should be minimal in terms 
///     of overall performance impact.
/// 
/// Maximum cage id determines how many processes can exist simultaneously in the RawPOSIX 
/// `Vec` in Rust is indexed using `usize` not `u64`
const MAX_CAGEID: usize = 1024; 

/// Pre-allocate MAX_CAGEID elements, all initialized to None.
/// Lazy causes `CAGE_MAP` to be initialized when it is first accessed, rather than when the program starts.
pub static CAGE_MAP: Lazy<RwLock<Vec<Option<Arc<Cage>>>>> = Lazy::new(|| {
    let mut vec = Vec::with_capacity(MAX_CAGEID);
    vec.resize_with(MAX_CAGEID, || None);
    RwLock::new(vec)
});

/// Add a cage to `CAGE_MAP` and map `cageid` to its index
pub fn add_cage(cageid: u64, cage: Cage) {
    let mut list = CAGE_MAP.write();
    if (cageid as usize) < MAX_CAGEID {
        list[cageid as usize] = Some(Arc::new(cage));
    } else {
        panic!("Cage ID exceeds MAX_CAGEID: {}", cageid);
    }
}

/// Delete the cage from `CAGE_MAP` by `cageid` as index
pub fn remove_cage(cageid: u64) {
    let mut list = CAGE_MAP.write();
    if (cageid as usize) < MAX_CAGEID {
        list[cageid as usize] = None;
    }
}

/// Get the cage's `Arc` reference via `cageid`
/// Error handling (when `Cage` is None) happens when calling
pub fn get_cage(cageid: u64) -> Option<Arc<Cage>> {
    let list = CAGE_MAP.read();
    if (cageid as usize) < MAX_CAGEID {
        list[cageid as usize].clone()
    } else {
        None
    }
}

/// Clear `CAGE_MAP` and exit all existing cages
pub fn cagetable_clear() {
    let mut exitvec = Vec::new();

    {
        let mut list = CAGE_MAP.write();
        for (cageid, cage) in list.iter_mut().enumerate() {
            if let Some(_c) = cage.take() {
                exitvec.push(cageid);
            }
        }
    }

    for cageid in exitvec {
        exit_syscall(
            cageid as u64, 
            sys_constants::EXIT_SUCCESS as u64,
            0, 0, 0, 0, 0,
        );
    }
}


/// Those functions are required by wasmtime to create the first cage. `verbosity` indicates whether 
/// detailed error messages will be printed if set 
pub fn lindrustinit(verbosity: isize) {
    let _ = VERBOSE.set(verbosity); //assigned to suppress unused result warning

    fdtables::register_close_handlers(
        fs_constants::FDKIND_KERNEL, 
        fdtables::NULL_FUNC, 
        kernel_close,
    );
    
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
    fdtables::get_specific_virtual_fd(0, fs_constants::STDIN_FILENO as u64, fs_constants::FDKIND_KERNEL, fs_constants::STDIN_FILENO as u64, false, 0).unwrap();
    // STDOUT
    fdtables::get_specific_virtual_fd(0, fs_constants::STDOUT_FILENO as u64, fs_constants::FDKIND_KERNEL, fs_constants::STDOUT_FILENO as u64, false, 0).unwrap();
    // STDERR
    fdtables::get_specific_virtual_fd(0, fs_constants::STDERR_FILENO as u64, fs_constants::FDKIND_KERNEL, fs_constants::STDERR_FILENO as u64, false, 0).unwrap();

    //init cage is its own parent
    let initcage = Cage {
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

    // Add cage to cagetable
    add_cage(
        0, // cageid
        initcage,
    );

    fdtables::init_empty_cage(1);
    // Set the first 3 fd to STDIN / STDOUT / STDERR
    // STDIN
    fdtables::get_specific_virtual_fd(1, fs_constants::STDIN_FILENO as u64, fs_constants::FDKIND_KERNEL, fs_constants::STDIN_FILENO as u64, false, 0).unwrap();
    // STDOUT
    fdtables::get_specific_virtual_fd(1, fs_constants::STDOUT_FILENO as u64, fs_constants::FDKIND_KERNEL, fs_constants::STDOUT_FILENO as u64, false, 0).unwrap();
    // STDERR
    fdtables::get_specific_virtual_fd(1, fs_constants::STDERR_FILENO as u64, fs_constants::FDKIND_KERNEL, fs_constants::STDERR_FILENO as u64, false, 0).unwrap();

}

pub fn lindrustfinalize() {
    cagetable_clear();
}
