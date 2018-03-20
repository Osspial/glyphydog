// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use ft::{FT_Memory, FT_MemoryRec_};

use std::os::raw::c_void;
use libc::{self, c_long};

pub fn alloc_mem_rec() -> *mut FT_MemoryRec_ {
    Box::into_raw(Box::new(FT_MemoryRec_ {
        user: 0 as *mut c_void,
        alloc: Some(ft_alloc),
        free: Some(ft_free),
        realloc: Some(ft_realloc)
    }))
}

unsafe extern "C" fn ft_alloc(_: FT_Memory, size: c_long) -> *mut c_void {
    libc::malloc(size as usize) as *mut c_void
}

unsafe extern "C" fn ft_free(_: FT_Memory, ptr: *mut c_void) {
    libc::free(ptr as *mut _);
}

unsafe extern "C" fn ft_realloc(_: FT_Memory, _: c_long, new_size: c_long, block: *mut c_void) -> *mut c_void {
    libc::realloc(block as *mut _, new_size as usize) as *mut c_void
}
