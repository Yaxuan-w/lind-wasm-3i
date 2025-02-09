/// This file is used for path conversion related functions 
use std::path::Component;
use crate::cage;
pub use std::ffi::{CString, CStr};
pub use std::{ptr, mem};
pub use libc::*;
use std::path::PathBuf;

pub use crate::constants::fs_constants;

pub fn convpath(cpath: &str) ->
    PathBuf {
    PathBuf::from(cpath)
}

pub fn normpath(origp: PathBuf, cageid: u64) -> PathBuf {
    let cage = cage::get_cage(cageid).unwrap();
    //If path is relative, prefix it with the current working directory, otherwise populate it with rootdir
    let mut newp = if origp.is_relative() {
        (**cage.cwd.read()).clone()
    } else {
        PathBuf::from("/")
    };

    for comp in origp.components() {
        match comp {
            //if we have a normal path component, push it on to our normed path
            Component::Normal(_) => {
                newp.push(comp);
            }

            //if we have a .. path component, pop the last component off our normed path
            Component::ParentDir => {
                newp.pop();
            }

            //if we have a . path component (Or a root dir or a prefix(?)) do nothing
            _ => {}
        };
    }
    newp
}

pub fn add_lind_root(cageid: u64, path: &str) -> CString {
    // Convert data type from &str into *const i8
    let relpath = normpath(convpath(path), cageid);
    let relative_path = relpath.to_str().unwrap();
    let full_path = format!("{}{}", fs_constants::LIND_ROOT, relative_path);
    let c_path = CString::new(full_path).unwrap();
    c_path
}


