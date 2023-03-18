//! The module/file/archive linker

use std::os::raw::c_char;

use super::prelude::*;

#[repr(C)]
#[derive(Debug)]
pub enum LLVMLinkerMode {
    LLVMLinkerDestroySource = 0,
    #[deprecated(since = "3.7.0", note = "LLVMLinkerPreserveSource has no effect")]
    LLVMLinkerPreserveSource_Removed = 1,
}

extern "C" {
    /// Link the source module into the destination module.
    ///
    /// Destroys the source module, returns true on error. Use the diagnostic
    /// handler to get any diagnostic message.
    pub fn LLVMLinkModules2(Dest: LLVMModuleRef, Src: LLVMModuleRef) -> LLVMBool;

    pub fn LLD_LinkElf(args: *const *const c_char, arglen: u32);
    pub fn LLD_LinkMachO(args: *const *const c_char, arglen: u32);
    pub fn LLD_LinkCoff(args: *const *const c_char, arglen: u32);
    pub fn LLD_LinkWasm(args: *const *const c_char, arglen: u32);
}
