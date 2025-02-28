//! Regular RawPOSIX will call through 
use crate::syscall_table::SYSCALL_TABLE;
use core::panic;
use dashmap::DashSet;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use parking_lot::RwLock;

/// ------------------------------------------------------------
/// `call_back` function is the dispatcher function for grate, so it's per grate bias (each grate will have same callback function, and 
/// threei calling different functions by different indexes). 
/// 
/// In this function, threei will add the mapping (grateid -> entry dispatcher function) in the 
pub fn threei_test_func<'a>(mut callback: Box<dyn FnMut(u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64, u64) -> i32 + 'a>) {
    // let open_result = callback(
    //     0, // index
    //     0, // cageid
    //     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    // println!("[3i|Wasm] -open- function returned in 3i: {}", open_result);
    // let add_result = callback(
    //     1, // index
    //     1, // cageid
    //     2, // arg1
    //     1, // arg1cageid
    //     3, // arg2
    //     0, // arg2cageid
    //     0, 0, 0, 0, 0, 0, 0, 0);
    // println!("[3i|Wasm] -add- function returned in 3i: {}", add_result);

    // add to GrateEntryTable
}
/// ------------------------------------------------------------

// use cage::cage::get_cage;
// use cage::memory::mem_helper::*;
use sysdefs::constants::threei_const;
// use sysdefs::constants::{PROT_READ, PROT_WRITE}; // might be used on memcp, so keep them for now

const exit_syscallnum: u64 = 30; // Develop purpose only

/// HANDLERTABLE:
/// <self_cageid, <callnum, (addr, dest_cageid)>
/// 1. callnum is the call that have access to execute syscall in addr -- acheive per syscall filter
/// 2. callnum is mapped to addr (callnum=addr) -- achieve per cage filter
///
/// In the current implementation, I only implemented per cage system call filtering.
/// Because in make_syscall, if we filter the system call based on per syscall, it will be difficult to track (because we
/// don’t know what the syscall num is that currently issues make)
/// 
/// Use Send to send it to another thread.
/// Use Sync to share between threads (T is Sync if and only if &T is Send).
pub type CallFunc = Arc<Mutex<dyn FnMut(
    u64, // Call index
    u64, // self cage id
    u64, // arg1
    u64, // arg1 cageid
    u64, u64, u64, u64, u64, u64, u64, u64, u64, u64
) -> i32 + Send + Sync>>;

pub type Raw_CallFunc = fn(
    target_cageid: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
    arg1_cageid: u64,
    arg2_cageid: u64,
    arg3_cageid: u64,
    arg4_cageid: u64,
    arg5_cageid: u64,
    arg6_cageid: u64,
) -> i32;

/// GrateEntryTable is to map entry dispatcher function per grateid.
const MAX_GRATEID: usize = 1024;
pub static GrateEntryTable: Lazy<RwLock<Vec<Option<CallFunc>>>> = Lazy::new(|| {
    let mut vec = Vec::with_capacity(MAX_GRATEID);
    vec.resize_with(MAX_GRATEID, || None);
    RwLock::new(vec)
});

fn get_grate_entry(grateid: u64) -> Option<CallFunc> {
    let table = GrateEntryTable.read();
    if (grateid as usize) < MAX_GRATEID {
        table[grateid as usize].clone() 
    } else {
        None
    }
}

// Keys are the grate, the value is a HashMap with a key of the callnum
// and the values are a (target_call_index, grate) tuple for the actual handlers...
// Added mutex to avoid race condition
lazy_static::lazy_static! {
    #[derive(Debug)]
    // <self_cageid, <callnum, (target_call_index, dest_grateid)>
    // callnum is mapped to addr, not self
    pub static ref HANDLERTABLE: Mutex<HashMap<u64, HashMap<u64, HashMap<u64, u64>>>> = Mutex::new(HashMap::new());
}

/// EXITING_TABLE
/// A grate/cage does not need to know the upper-level grate/cage information, but only needs to manage where the call goes.
/// I use a global variable table to represent the cage/grate that is exiting. This table will be removed after the corresponding
/// grate/cage performs exit_syscall. During the execution of the corresponding operation, all other 3i calls that want to operate
/// the corresponding syscall will be blocked (additional check)
/// Only initialize once, and using dashset to support higher performance in high concurrency needs
static EXITING_TABLE: Lazy<DashSet<u64>> = Lazy::new(|| DashSet::new());

/// This function is used to register a syscall with what permissions it will have to call other system calls.
///
/// For example:
/// I want cage 7 to have system call 34 call into my cage's function foo 
/// 
/// ```
/// register_handler(
///     NOTUSED, 7,  34, NOTUSED,
///    foo, mycagenum,
///    ...)
/// ```
/// 
/// TODO: 
/// 1. match-all / deregister cases 
/// 2. handle treat as function ptr not index (data structure will change)
pub fn register_handler(
    _callnum: u64,
    targetcage: u64,    // Cage to modify
    targetcallnum: u64, // Syscall number or match-all indicator
    _arg1cage: u64,
    handlefunc: u64,     // Function index to register (for grate, also called destination call) _or_ 0 for deregister 
    handlefunccage: u64, // Grate cage id _or_ Deregister flag or additional information
    _arg3: u64,
    _arg3cage: u64,
    _arg4: u64,
    _arg4cage: u64,
    _arg5: u64,
    _arg5cage: u64,
    _arg6: u64,
    _arg6cage: u64,
) -> i32 {
    // Make sure that both the cage that registers the handler and the cage being registered are valid (not in exited state)
    if EXITING_TABLE.contains(&targetcage) && EXITING_TABLE.contains(&handlefunccage) {
        return threei_const::ELINDESRCH as i32;
    }

    let mut handler_table = HANDLERTABLE.lock().unwrap();

    if let Some(cage_entry) = handler_table.get(&targetcage) {
        // Check if targetcallnum exists
        if let Some(callnum_entry) = cage_entry.get(&targetcallnum) {
            // Check if handlefunc exists
            match callnum_entry.get(&handlefunc) {
                Some(existing_dest_grateid) if *existing_dest_grateid == handlefunccage => return 0, // Do nothing
                Some(_) => panic!("Already exists"),
                None => {} // If `handlefunc` not exists, execute insertion
            }
        }
    }
    
    handler_table
        .entry(targetcage)
        .or_insert_with(HashMap::new)
        .entry(targetcallnum)
        .or_insert_with(HashMap::new)
        .insert(handlefunc, handlefunccage);
    0
}

/// This copies the handler table used by a cage to another cage.  
/// This is often useful for calls like fork, so that a grate can later
/// add or remove entries.
///
/// Note that this call is itself made through a syscall and is thus
/// interposable.
// pub fn copy_handler_table_to_cage(
//     _callnum: u64,
//     targetcage: u64,
//     srccage: u64,
//     _arg1cage: u64,
//     _arg2: u64,
//     _arg2cage: u64,
//     _arg3: u64,
//     _arg3cage: u64,
//     _arg4: u64,
//     _arg4cage: u64,
//     _arg5: u64,
//     _arg5cage: u64,
//     _arg6: u64,
//     _arg6cage: u64,
// ) -> u64 {
//     let mut handler_table = HANDLERTABLE.lock().unwrap();

//     if let Some(srccage_entries) = handler_table.get(&srccage) {
//         // Create new HashMap for target case
//         let mut new_entries = HashMap::new();

//         for (callnum, cage_call_table) in srccage_entries {
//             let new_cage_call_table = Arc::new(Mutex::new({
//                 // Deep copy CageCallTable
//                 if let Ok(src_cage_call_table) = cage_call_table.lock() {
//                     CageCallTable {
//                         defaultcallfunc: src_cage_call_table
//                             .defaultcallfunc
//                             .as_ref()
//                             .map(|funcs| funcs.clone()),
//                         thiscalltable: src_cage_call_table.thiscalltable.clone(),
//                     }
//                 } else {
//                     continue;
//                 }
//             }));

//             new_entries.insert(*callnum, new_cage_call_table);
//         }

//         handler_table.insert(targetcage, new_entries);

//         println!(
//             "Successfully copied handler table entries from cage {} to cage {}",
//             srccage, targetcage
//         );
//     } else {
//         println!("No entries found for srccage {} in HANDLERTABLE", srccage);
//         return threei_const::ELINDAPIABORTED;
//     }
//     0
// }

/// `make_syscall` is simpler, which is to directly execute the system call that grate/cage wants to execute.
/// But there are several special cases that need to be treated differently:
///
/// The first is that when the target grate/cage is executing exit(), all system calls to the target grate/cage
/// should directly return ELINDESRCH (the process does not exist).
///
/// The second is that when `srccage` and `targetcage` are different, we need to verify whether `srccage` has the permission
/// to issue a system call marked as callnum to the target cage/grate by checking the dependencies in `HANDLERTABLE`.
///
/// The third case is more direct. When `srccage` and `targetcage` are the same, we do not need to check (because
/// there is always permission). The only thing that needs to be distinguished is that `exit()` cannot be called. If a
/// cage/grate wants to execute `exit()` to themselves, they need to call `trigger_harsh_exit()` to mark them as `EXITING` status
///
/// To distinguish whether the call is from a grate or a cage, we utilize the feature of different number of arguments
/// If the call is from a cage:
///     - only
/// If the call is from a grate:
///     -
///     - additional arguments will be args corresponding with different
///
/// TODO:
/// - confirm the return type
/// - Do we need to pass self_syscallnum?? -if not how to perform permission check? -only perform syscall filter per cage
pub fn make_syscall(
    self_cageid: u64, // is required to get the cage instance 
    syscall_num: u64,
    target_cageid: u64,
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
    println!("[3i|make_syscall] syscallnum: {}, self_cageid: {}, target_cageid: {}", syscall_num, self_cageid, target_cageid);
    // Return error if the target cage/grate is exiting. We need to add this check beforehead, because make_syscall will also
    // contain cases that can directly redirect a syscall when self_cageid == target_id, which will bypass the handlertable check
    if EXITING_TABLE.contains(&target_cageid) && syscall_num != exit_syscallnum {
        return threei_const::ELINDESRCH as i32;
    }

    // Regular case (call from cage to rawposix)
    if self_cageid == target_cageid || syscall_num == exit_syscallnum {
        // println!("syscall num in make_syscall: {:?}", syscall_num);
        if let Some(&(_, syscall_func)) = SYSCALL_TABLE.iter().find(|&&(num, _)| num == syscall_num)
        {
            let ret = syscall_func(
                target_cageid,
                arg1,
                arg1_cageid,
                arg2,
                arg2_cageid,
                arg3,
                arg3_cageid,
                arg4,
                arg4_cageid,
                arg5,
                arg5_cageid,
                arg6,
                arg6_cageid,
            );
            eprintln!("[3i|make_syscall] syscallnum: {}, ret: {}, self_cageid: {}, target_cageid: {}", syscall_num, ret, self_cageid, target_cageid);
            return ret;
        } else {
            eprintln!("[3i|make_syscall] Syscall number {} not found!", syscall_num);
            return threei_const::ELINDAPIABORTED as i32;
        }
    }

    0
    // let handler_table = HANDLERTABLE.lock().unwrap();
    // If selfcageid != targetcageid --> check the syscall handler table (since here's the cage of grate / dependencies)
    // let (call_index, grateid) = handler_table.get(&self_cageid).and_then(|sub_table| sub_table.get(&syscall_num)).copied().unwrap();
    // get_grate_entry(grateid);
}

/***************************** trigger_harsh_cage_exit & harsh_cage_exit *****************************/

/// Starts an unclean exit process for the target cage. Notifies threei and related grates to quickly block
/// new calls by adding to EXITING_TABLE and clean up resources. The call is only called from trusted modules
/// or system kernel so we don't need selfcageid to check (we will remove from cage table directly)
///
/// TODO:
/// We want: This function cannot be called directly by user mode to ensure that it is only triggered by the
/// system kernel or trusted modules
/// Question: How we check the call is only called from trusted mode..?
// pub fn trigger_harsh_cage_exit(targetcage: u64, exittype: u64) {
//     // Use {} to specific the lock usage to avoid dead lock
//     {
//         let mut handler_table = HANDLERTABLE.lock().unwrap();
//         // Remove exited cage entry from syscall handler
//         if handler_table.remove(&targetcage).is_none() {
//             panic!(
//                 "targetcage {:?} entry not found in HANDLERTABLE when triggering harsh cage exit.",
//                 targetcage
//             );
//         }
//     }

//     {
//         EXITING_TABLE.insert(targetcage);
//         // println!("Added targetcage {} to EXITING_TABLE", targetcage);
//     }

//     // TODO: replace call num with real exit_syscall num
//     harsh_cage_exit(
//         exit_syscallnum, // exit_syscall
//         targetcage,
//         exittype,
//         0,
//         0,
//         0,
//         0,
//         0,
//         0,
//         0,
//         0,
//         0,
//         0,
//         0,
//     );

//     // The block of code is enclosed within curly braces to explicitly scope the lock on the `HANDLERTABLE`,
//     // which ensures that the lock is released as soon as the operation within the block is completed.
//     {
//         let mut handler_table = HANDLERTABLE.lock().unwrap();
//         // Update syscall handler to remove all access to exited cage
//         handler_table.retain(|_self_cageid, callmap| {
//             callmap.retain(|_callnum, cage_calltable| {
//                 if let Ok(mut cage_calltable) = cage_calltable.lock() {
//                     // Remove entries in `thiscalltable` where the destination cage ID matches `targetcage`
//                     cage_calltable
//                         .thiscalltable
//                         .retain(|&dest_cageid, _| dest_cageid != targetcage);

//                     // Check if both `thiscalltable` and `defaultcallfunc` are empty
//                     !(cage_calltable.thiscalltable.is_empty()
//                         && cage_calltable.defaultcallfunc.is_none())
//                 } else {
//                     // If we can't acquire the lock, keep the entry
//                     true
//                 }
//             });

//             // Retain the `callmap` only if it still contains entries
//             !callmap.is_empty()
//         });
//     }
// }

// pub fn harsh_cage_exit(
//     callnum: u64,    // System call number (can be used if called as syscall)
//     targetcage: u64, // Cage to cleanup
//     exittype: u64,   // Exit type (e.g., fault, manual exit)
//     _arg1cage: u64,
//     _arg2: u64,
//     _arg2cage: u64,
//     _arg3: u64,
//     _arg3cage: u64,
//     _arg4: u64,
//     _arg4cage: u64,
//     _arg5: u64,
//     _arg5cage: u64,
//     _arg6: u64,
//     _arg6cage: u64,
// ) -> u64 {
//     // Directly execute
//     let result = make_syscall(targetcage, callnum, targetcage, exittype, targetcage, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);

//     // TODO:
//     // This should align with specific exit type. Does different exit type mean different things..?
//     // aka do we need to handle different situations here?
//     if result != exittype as i32 {
//         panic!("Error on exit() {}", result);
//     }

//     {
//         EXITING_TABLE.remove(&targetcage);
//         // println!("Added targetcage {} to EXITING_TABLE", targetcage);
//     }

//     0
// }

/***************************** copy_data_between_cages *****************************/
// Validate the memory range for both source (`srcaddr -> srcaddr + srclen`) and destination (`destaddr -> destaddr + destlen`)
// using the corresponding `vmmap` functions in RawPOSIX.
//
// First, check if the source range is valid and properly mapped.
// Then, check if the destination range is valid:
//  - If any part of the destination range is unmapped, attempt to map it using the appropriate `vmmap` function.
//  - If the destination range becomes valid and satisfies the required permissions after mapping, proceed to
//      perform the copy operation.
// Otherwise, abort the operation if the mapping fails or permissions are insufficient.
// pub fn copy_data_between_cages(
//     callnum: u64,
//     targetcage: u64,
//     srcaddr: u64,
//     srccage: u64,
//     destaddr: u64,
//     destcage: u64,
//     len: u64,
//     _arg3cage: u64,
//     copytype: u64,
//     _arg4cage: u64,
//     _arg5: u64,
//     _arg5cage: u64,
//     _arg6: u64,
//     _arg6cage: u64,
// ) -> u64 {
//     // Check address validity and permissions
//     // Validate source address
//     if !check_addr(srccage, srcaddr, len as usize, PROT_READ as i32).unwrap_or(false) {
//         eprintln!("Source address is invalid.");
//         return threei_const::ELINDAPIABORTED; // Error: Invalid address
//     }

//     // Validate destination address, and we will try to map if we don't the memory region
//     // unmapping
//     if !check_addr(destcage, destaddr, len as usize, PROT_WRITE as i32).unwrap_or(false) {
//         eprintln!("Dest address is invalid.");
//         return threei_const::ELINDAPIABORTED; // Error: Invalid address
//     }

//     // TODO:
//     //  - Do we need to consider the permission relationship between cages..?
//     //      ie: only parent cage can perfrom copy..?
//     // if !_has_permission(srccage, destcage) {
//     //     eprintln!("Permission denied between cages.");
//     //     return threei_const::ELINDAPIABORTED; // Error: Permission denied
//     // }

//     // Perform the data copy
//     unsafe {
//         let src_ptr = srcaddr as *const u8;
//         let dest_ptr = destaddr as *mut u8;
//         std::ptr::copy_nonoverlapping(src_ptr, dest_ptr, len as usize);
//     }

//     0
// }

// -- Check if permissions allow data copying between cages
// TODO:
// How we handle permission relationship...?
// fn _has_permission(srccage: u64, destcage: u64) -> bool {
//     lazy_static::lazy_static! {
//         static ref PERMISSION_TABLE: Mutex<HashMap<u64, HashSet<u64>>> = Mutex::new(HashMap::new());
//     }

//     // Check permission
//     let permission_table = PERMISSION_TABLE.lock().unwrap();
//     if let Some(allowed_destinations) = permission_table.get(&srccage) {
//         if allowed_destinations.contains(&destcage) {
//             return true;
//         } else {
//             eprintln!(
//                 "Permission denied: Cage {} cannot access Cage {}.",
//                 srccage, destcage
//             );
//             return false;
//         }
//     }
//     false
// }

// ---- CODE BELOW IS HELPER FUNCTIONS FOR TESTING ----
pub fn testing_remove_cage_entry(target_cageid: u64) -> i32 {
    let mut handler_table = HANDLERTABLE.lock().unwrap();
    if handler_table.remove(&target_cageid).is_none() {
        eprintln!(
            "targetcage {:?} entry not found in HANDLERTABLE when triggering harsh cage exit",
            target_cageid
        );
        return -1;
    }
    return 0;
}

pub fn testing_remove_all() {
    let mut handler_table = HANDLERTABLE.lock().unwrap();
    handler_table.clear();
}
