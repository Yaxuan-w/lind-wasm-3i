use crate::threei::syscall_table::SYSCALL_TABLE;
use crate::threei::threeiconstant;
use core::panic;
use std::collections::HashMap;
use dashmap::DashSet;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

use tracing::{info, instrument};

/// HANDLERTABLE:
/// <self_cageid, <callnum, (addr, dest_cageid)>
/// 1. callnum is the call that have access to execute syscall in addr -- acheive per syscall filter
/// 2. callnum is mapped to addr (callnum=addr) -- achieve per cage filter 
/// 
/// In the current implementation, I only implemented per cage system call filtering. 
/// Because in make_syscall, if we filter the system call based on per syscall, it will be difficult to track (because we 
/// don’t know what the syscall num is that currently issues make)

pub type CallFunc = fn(
    target_cageid:u64,
    arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64, arg6: u64,
) -> i32;

#[derive(Debug, Clone)]
pub struct CageCallTable {
    pub defaultcallfunc: Option<HashMap<u64, CallFunc>>,
    pub thiscalltable: HashMap<u64, CallFunc>,   // <target_cageid, jump address> 
}

impl CageCallTable {
    pub fn new(initial_entries: Vec<(u64, CallFunc)>) -> Self {
        let mut thiscalltable = HashMap::new();
        for (cageid, callfunc) in initial_entries {
            thiscalltable.insert(cageid, callfunc);
        }
        Self {
            defaultcallfunc: None,
            thiscalltable,
        }
    }

    // This function will only be called when MATCHALL flag has been set in register_handler function
    // to initialize default
    pub fn set_default_handler(&mut self, targetcage: u64) -> Result<(), Box<dyn std::error::Error>> {
        let mut default_mapping = HashMap::new();
        for &(_, syscall_name) in SYSCALL_TABLE {
            default_mapping.insert(targetcage, syscall_name);
        }
        self.defaultcallfunc = Some(default_mapping);
        return Ok(());
    }
}

// Keys are the cage, the value is a HashMap with a key of the callnum
// and the values are a (addr, cage) tuple for the actual handlers...
// Added mutex to avoid race condition
lazy_static::lazy_static! {
    #[derive(Debug)]
    // <self_cageid, <callnum, (addr, dest_cageid)>
    // callnum is mapped to addr, not self 
    pub static ref HANDLERTABLE: Mutex<HashMap<u64, HashMap<u64, Arc<Mutex<CageCallTable>>>>> = Mutex::new(HashMap::new());
}


/// EXITING_TABLE
/// A grate/cage does not need to know the upper-level grate/cage information, but only needs to manage where the call goes.
/// I use a global variable table to represent the cage/grate that is exiting. This table will be removed after the corresponding 
/// grate/cage performs exit_syscall. During the execution of the corresponding operation, all other 3i calls that want to operate 
/// the corresponding syscall will be blocked (additional check)
/// Only initialize once, and using dashset to support higher performance in high concurrency needs 
static EXITING_TABLE: Lazy<DashSet<u64>> = Lazy::new(|| DashSet::new());

/// This function is used to register a syscall with what permissions it will have to call other system calls. 
/// There are a few special cases to note:
/// 
/// If targetcallnum is THREEI_MATCHALL, the target operation is applied to all syscalls in the syscall table.
/// - If handlefunccage is THREEI_DEREGISTER, all items in HANDLERTABLE (note: <self_cageid, <callnum, (addr, dest_cageid)>) 
///     that match callnum=targetcallnum and dest_cageid=targetcage are removed.
/// - If handlefunccage is not set, all syscalls in the syscall table are added to `defaultfunc` of `targetcage`
/// 
/// If THREEI_MATCHALL is not set, the thief adds the corresponding items according to the passed arguments
/// 
/// TODO:
/// Differences between callnum and handlefunc...?
pub fn register_handler(
    _callnum: u64,                  
    targetcage: u64,                // Cage to modify
    targetcallnum: u64,             // Syscall number or match-all indicator
    _arg1cage: u64,                 
    handlefunc: u64,                // Function to register or 0 for deregister !!!!
    handlefunccage: u64,            // Deregister flag or additional information
    _arg3: u64,                      
    _arg3cage: u64,                  
    _arg4: u64,                      
    _arg4cage: u64,                  
    _arg5: u64,                      
    _arg5cage: u64,                 
    _arg6: u64,                    
    _arg6cage: u64,                 
) -> u64 {
    // Make sure that both the cage that registers the handler and the cage being registered are valid (not in exited state)
    if EXITING_TABLE.contains(&targetcage) && EXITING_TABLE.contains(&handlefunccage){
        return threeiconstant::ELINDESRCH;
    }

    let mut handler_table = HANDLERTABLE.lock().unwrap();

    if handlefunccage == threeiconstant::THREEI_DEREGISTER {
        if targetcallnum == threeiconstant::THREEI_MATCHALL {
            // Remove all handlers where dest_cageid == targetcage
            handler_table.retain(|_self_cageid, inner_map| {
                inner_map.retain(|_callnum, cage_call_table| {
                    let mut cage_call_table = cage_call_table.lock().unwrap();
                    
                    // Remove entries from `thiscalltable` 
                    cage_call_table.thiscalltable.retain(|&key, _| key != targetcage);
                    
                    // Remove entries from `defaultcallfunc` 
                    if let Some(default_callfunc_map) = &mut cage_call_table.defaultcallfunc {
                        default_callfunc_map.retain(|&key, _| key != targetcage);
                    }
                    
                    // Retain `cage_call_table` only if it still has relevant entries
                    !cage_call_table.thiscalltable.is_empty()
                        || cage_call_table
                            .defaultcallfunc.as_ref()
                            .map_or(false, |map| !map.is_empty())
                });
                // Retain `inner_map` only if it still has relevant entries
                !inner_map.is_empty()
            });
        } else {
            // Remove specific handler by keeping the item whose callnum != targetcallnum && dest_cageid != targetcage 
            handler_table.retain(|_self_cageid, inner_map| {
                inner_map.retain(|&callnum, cage_call_table| {
                    let mut cage_call_table = cage_call_table.lock().unwrap(); 
                    // Check the `thiscalltable` for entries matching `targetcallnum` and `targetcage`
                    let should_retain_this = !cage_call_table.thiscalltable.contains_key(&targetcage) 
                                        || callnum != targetcallnum;
                    if !should_retain_this {
                        cage_call_table.thiscalltable.remove(&targetcage); 
                    }
                    if let Some(default_callfunc_map) = &mut cage_call_table.defaultcallfunc {
                        default_callfunc_map.retain(|&key, _| key != targetcage);
                    }
                    // Retain only if `thiscalltable` and `defaultcallfunc` are both not empty
                    !cage_call_table.thiscalltable.is_empty()
                        || cage_call_table
                            .defaultcallfunc.as_ref()
                            .map_or(false, |map| !map.is_empty())
                });
                // Remove the outer entry if the inner map is empty
                !inner_map.is_empty()
            });
        }
    } else {
        let cage_handlers = handler_table.entry(handlefunccage).or_insert_with(HashMap::new);

        if targetcallnum == threeiconstant::THREEI_MATCHALL {
            // Get the entry
            let cage_call_table = cage_handlers
                .entry(targetcallnum)
                .or_insert_with(|| Arc::new(Mutex::new(CageCallTable::new(vec![]))));
            let mut cage_call_table = cage_call_table.lock().unwrap();
            match cage_call_table.set_default_handler(targetcage) {
                Ok(_) => return 0,
                Err(_e) => return threeiconstant::ELINDAPIABORTED,
            };
        }

        // Find the corresponding CallFunc pointer from SYSCALL_TABLE
        if let Some(&(_, syscall_func)) = SYSCALL_TABLE.iter().find(|&&(num, _)| num == targetcallnum) {
            let new_cagetable = CageCallTable::new(vec![(targetcage, syscall_func)]);
            cage_handlers.insert(handlefunc, Arc::new(Mutex::new(new_cagetable)));
        } else {
            eprintln!("Syscall number {} not found in SYSCALL_TABLE!", targetcallnum);
            return threeiconstant::ELINDAPIABORTED; // Error: Syscall not found
        }
    }
    // eprintln!("HANDLERTABLE: {:?}", *handler_table);
    0 
}

/// This copies the handler table used by a cage to another cage.  
/// This is often useful for calls like fork, so that a grate can later 
/// add or remove entries.
///
/// Note that this call is itself made through a syscall and is thus 
/// interposable.
pub fn copy_handler_table_to_cage(
    _callnum:u64, targetcage:u64, 
    srccage:u64, _arg1cage:u64,
    _arg2:u64, _arg2cage:u64,
    _arg3:u64, _arg3cage:u64,
    _arg4:u64, _arg4cage:u64,
    _arg5:u64, _arg5cage:u64,
    _arg6:u64, _arg6cage:u64, 
) -> u64 {
    let mut handler_table = HANDLERTABLE.lock().unwrap();

    if let Some(srccage_entries) = handler_table.get(&srccage) {
        // Create new HashMap for target case
        let mut new_entries = HashMap::new();

        for (callnum, cage_call_table) in srccage_entries {
            let new_cage_call_table = Arc::new(Mutex::new({
                // Deep copy CageCallTable
                if let Ok(src_cage_call_table) = cage_call_table.lock() {
                    CageCallTable {
                        defaultcallfunc: src_cage_call_table
                            .defaultcallfunc
                            .as_ref()
                            .map(|funcs| funcs.clone()), 
                        thiscalltable: src_cage_call_table.thiscalltable.clone(), 
                    }
                } else {
                    continue;
                }
            }));

            new_entries.insert(*callnum, new_cage_call_table);
        }

        handler_table.insert(targetcage, new_entries);

        println!(
            "Successfully copied handler table entries from cage {} to cage {}",
            srccage, targetcage
        );
    } else {
        println!("No entries found for srccage {} in HANDLERTABLE", srccage);
        return threeiconstant::ELINDAPIABORTED;
    }
    0
}

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
/// TODO: confirm the return type 
#[instrument]
pub fn make_syscall(
    self_cageid: u64, 
    self_syscallnum: u64,
    syscall_num: u64, 
    target_cageid: u64, 
    arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64, arg6: u64,
) -> i32 {
    info!("Executed make_syscall");

    // Return error if the target cage/grate is exiting. We need to add this check beforehead, because make_syscall will also 
    // contain cases that can directly redirect a syscall when self_cageid == target_id, which will bypass the handlertable check
    // TODO: replace 3 with actual exit callnum
    if EXITING_TABLE.contains(&target_cageid) && syscall_num != 3 {
        return threeiconstant::ELINDESRCH as i32;
    }

    // TODO: replace 3 with actual exit callnum
    if self_cageid == target_cageid || syscall_num == 3 {
        // println!("syscall num in make_syscall: {:?}", syscall_num);
        if let Some(&(_, syscall_func)) = SYSCALL_TABLE.iter().find(|&&(num, _)| num == syscall_num) {
            return syscall_func(
                target_cageid,
                arg1, arg2, arg3, arg4, arg5, arg6,
            );
        } else {
            eprintln!("Syscall number {} not found!", syscall_num);
            return threeiconstant::ELINDAPIABORTED as i32; 
        }
    }

    let table_lock = HANDLERTABLE.lock().unwrap(); 
    // If selfcageid != targetcageid --> check the syscall handler table (since here's the cage of grate / dependencies)
    // Find the HashMap corresponding to `self_cageid`.
    if let Some(call_map) = table_lock.get(&self_cageid) {
        // Find the Arc<Mutex<CageCallTable>> corresponding to `self_syscallnum`.
        if let Some(cage_call_table_arc) = call_map.get(&self_syscallnum) {
            let cage_call_table = cage_call_table_arc.lock().unwrap(); // Lock the CageCallTable
            // Find the CallFunc for `target_cageid` in `thiscalltable`.
            // TODO:
            // - How to deal with multiple syscalls with same target cage num?
            if let Some(syscall_func) = cage_call_table.thiscalltable.get(&target_cageid).cloned() {
                // eprintln!("self cage id = {:?}, target cage id = {:?}", self_cageid, target_cageid);
                return syscall_func(
                    target_cageid,
                    arg1, arg2, arg3, arg4, arg5, arg6,
                );
            } else {
                return threeiconstant::ELINDESRCH as i32; 
            };
        } else {
            eprintln!("NO target syscall {} found for self cage {}", syscall_num, self_cageid);
            return threeiconstant::ELINDAPIABORTED as i32;
        }
    } else {
        info!("Arrive the end of make_syscall");
        eprintln!("Permission denied! No syscalls alllowed for self cage {}", self_cageid);
        return threeiconstant::ELINDAPIABORTED as i32;
    }
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
#[instrument]
pub fn trigger_harsh_cage_exit(
    targetcage: u64, 
    exittype: u64,
) {
    info!("Executed trigger_harsh_cage_exit");

    // Use {} to specific the lock usage to avoid dead lock
    {
        let mut handler_table = HANDLERTABLE.lock().unwrap();
        // Remove exited cage entry from syscall handler
        if handler_table.remove(&targetcage).is_none() {
            panic!("targetcage {:?} entry not found in HANDLERTABLE when triggering harsh cage exit.", targetcage);
        }
    }

    {
        EXITING_TABLE.insert(targetcage);
        // println!("Added targetcage {} to EXITING_TABLE", targetcage);
    }

    // TODO: replace call num with real exit_syscall num
    harsh_cage_exit(
        3, // exit_syscall
        targetcage, 
        exittype, 
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    );

    // The block of code is enclosed within curly braces to explicitly scope the lock on the `HANDLERTABLE`,
    // which ensures that the lock is released as soon as the operation within the block is completed. 
    {
        let mut handler_table = HANDLERTABLE.lock().unwrap();
        // Update syscall handler to remove all access to exited cage
        handler_table.retain(|_self_cageid, callmap| {
            callmap.retain(|_callnum, cage_calltable| {
                if let Ok(mut cage_calltable) = cage_calltable.lock() {
                    // Remove entries in `thiscalltable` where the destination cage ID matches `targetcage`
                    cage_calltable
                        .thiscalltable
                        .retain(|&dest_cageid, _| dest_cageid != targetcage);
    
                    // Check if both `thiscalltable` and `defaultcallfunc` are empty
                    !(cage_calltable.thiscalltable.is_empty() && cage_calltable.defaultcallfunc.is_none())
                } else {
                    // If we can't acquire the lock, keep the entry
                    true
                }
            });
    
            // Retain the `callmap` only if it still contains entries
            !callmap.is_empty()
        });
    }
    
    info!("Arrive the end of trigger_harsh_cage_exit");
}

#[instrument]
pub fn harsh_cage_exit(
    callnum:u64,    // System call number (can be used if called as syscall)
    targetcage:u64, // Cage to cleanup
    exittype:u64,   // Exit type (e.g., fault, manual exit)
    _arg1cage:u64,
    _arg2:u64, 
    _arg2cage:u64,
    _arg3:u64, 
    _arg3cage:u64,
    _arg4:u64, 
    _arg4cage:u64,
    _arg5:u64, 
    _arg5cage:u64,
    _arg6:u64, 
    _arg6cage:u64, 
) -> u64 {
    info!("Executed harsh_cage_exit");

    // Directly execute 
    let result = make_syscall(
        targetcage,
        targetcage, 
        callnum, 
        targetcage, 
        exittype,
         0, 0, 0, 0, 0);

    // TODO:
    // This should align with specific exit type. Does different exit type mean different things..?
    // aka do we need to handle different situations here?
    if result != exittype as i32 {
        panic!("Error on exit() {}", result);
    }
    
    {
        EXITING_TABLE.remove(&targetcage);
        // println!("Added targetcage {} to EXITING_TABLE", targetcage);
    }

    info!("Arrive the end of harsh_cage_exit");
    0
}

// ---- CODE BELOW IS HELPER FUNCTIONS FOR TESTING ----
pub fn testing_remove_cage_entry(target_cageid: u64) -> i32 {
    let mut handler_table = HANDLERTABLE.lock().unwrap();
    if handler_table.remove(&target_cageid).is_none() {
        eprintln!("targetcage {:?} entry not found in HANDLERTABLE when triggering harsh cage exit", target_cageid);
        return -1;
    }
    return 0;
}

pub fn testing_remove_all() {
    let mut handler_table = HANDLERTABLE.lock().unwrap();
    handler_table.clear();
}
// ---- CODE BELOW WILL BE TESTED WITH VMMAP ----
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
//     callnum:u64, 
//     targetcage:u64, 
//     srcaddr:u64, 
//     srccage:u64,
//     destaddr:u64, 
//     destcage:u64,
//     len:u64, 
//     _arg3cage:u64,
//     copytype:u64, 
//     _arg4cage:u64,
//     _arg5:u64, 
//     _arg5cage:u64,
//     _arg6:u64, 
//     _arg6cage:u64
// ) -> u64 {
//     // Check address validity and permissions 
//     // Validate source address
//      if !_validate_addr(srccage, srcaddr, len, PROT_READ as u64).unwrap_or(false) {
//         eprintln!("Source address is invalid.");
//         return threeiconstant::ELINDAPIABORTED; // Error: Invalid address
//     }

//     // Validate destination address, and we will try to map if we don't the memory region 
//     // unmapping
//     if !_validate_addr(destcage, destaddr, len, PROT_WRITE as u64).unwrap_or(false) {
//         if !_attemp_dest_mapping(destcage, destaddr, len, PROT_WRITE as u64).unwrap_or(false) {
//             eprintln!("Failed to map destination address.");
//             return threeiconstant::ELINDAPIABORTED; // Error: Mapping Failed
//         }
//     }

//     // TODO:
//     //  - Do we need to consider the permission relationship between cages..? 
//     //      ie: only parent cage can perfrom copy..?
//     // if !_has_permission(srccage, destcage) {
//     //     eprintln!("Permission denied between cages.");
//     //     return threeiconstant::ELINDAPIABORTED; // Error: Permission denied
//     // }

//     // Perform the data copy
//     unsafe {
//         match copytype {
//             0 => { // Raw memory copy
//                 let src_ptr = srcaddr as *const u8;
//                 let dest_ptr = destaddr as *mut u8;
//                 std::ptr::copy_nonoverlapping(src_ptr, dest_ptr, len as usize);
//             }
//             1 => { // Null-terminated string copy
//                 let src_ptr = srcaddr as *const u8;
//                 let dest_ptr = destaddr as *mut u8;
//                 for i in 0..len {
//                     let byte = *src_ptr.add(i as usize);
//                     *dest_ptr.add(i as usize) = byte;
//                     if byte == 0 {
//                         break;
//                     }
//                 }
//             }
//             _ => {
//                 eprintln!("Unsupported copy type: {}", copytype);
//                 return threeiconstant::ELINDAPIABORTED; // Error: Unsupported copy type
//             }
//         }
//     }

//     0
// }

// Helper function for copy_data_between_cages 
// Validates whether the specified memory range is valid, mapped, and meets the required
// permissions for the given cage. Ensure addr + len does not wrap or exceed bounds
// Return type as `Result` is used to distinguish whether the operation failed because the logic verification 
// failed (such as illegal address) or other errors occurred during program operation (such as system call failure)
// fn _validate_addr(
//     cage_id: u64,
//     addr: u64,
//     len: u64,
//     required_prot: u64,
// ) -> Result<bool, io::Error> {
//     let cage = interface::cagetable_getref(cage_id);
//     let rawposix_vmmap = cage.vmmap.read(); 
//     // Get the end address for validation
//     let end_addr = addr.checked_add(len).expect("Address computation overflowed");
//     // Get the base address of the cage and compute the cage valide address range
//     // Memory region per cage = 2**64
//     // TODO: Add check for unwrap
//     let baseaddr = rawposix_vmmap.base_address.unwrap() as u64;
//     let max_addr = baseaddr.checked_add(1 << 64).expect("Address computation overflowed");

//     if addr < baseaddr || end_addr > max_addr {
//         return Ok(false); // Address exceeds the cage's valid range
//     }

//     let start_page = addr >> 12;
//     let end_page = (addr + len - 1) >> 12;

//     let req_prot = required_prot as i32;
//     for const { page as i32 } in start_page..=end_page {
//         if let Some(entry) = rawposix_vmmap.find_page(page) {
//             if entry.cage_id != cage_id || entry.prot & req_prot != req_prot {
//                 return Ok(false);
//             }
//         } else {
//             return Ok(false); 
//         }
//     }

//     Ok(true)
// }

// fn _attemp_dest_mapping(
//     cage_id: u64,
//     addr: u64,
//     len: u64,
//     required_prot: u64,
// ) -> Result<bool, io::Error> {
//     let cage = interface::cagetable_getref(cage_id);
//     let mut rawposix_vmmap = cage.vmmap.write(); 
//     let start_page = addr >> 12;
//     let end_page = (addr + len - 1) >> 12;

//     // Because we are not sure whether all the pages from destaddr to destaddr+len are mapped, 
//     // we loop each page to check and try to map them.
//     for const { page as i32 } in start_page..=end_page {
//         if rawposix_vmmap.find_page(page).is_none() {
//             let new_entry = VmmapEntry::new(
//                 page,
//                 1,                       
//                 required_prot as i32,    
//                 required_prot as i32,    
//                 MAP_PRIVATE as i32,             
//                 false,                   // removed = false
//                 0,                      
//                 0,                       
//                 cage_id,                
//                 MemoryBackingType::Anonymous, 
//             );

//             rawposix_vmmap.add_entry(new_entry);
//         }
//     }

//     Ok(true)
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