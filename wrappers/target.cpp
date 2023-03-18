/* llvm-c/Target.h helper functions wrappers.
 *
 * The LLVMInitializeAll* functions and friends are defined `static inline`, so
 * we can't bind directly to them (the function body is generated via macro),
 * so here are some wrappers.
 */
#include "target.h"

void _LLD_Link(const char** rawArgs, uint32_t arglen, uint32_t linkType) {
    // ld.lld => ELF (Unix)
    // ld64.lld => Mach-O (MacOS)
    // lld-link => COFF (Windows)
    // wasm-ld => WASM (Wasm)

    std::vector<const char*> args(rawArgs, rawArgs + arglen);
    if (linkType == 0) {
        args.insert(args.begin(), "ld.lld");
        lld::elf::link(args, llvm::outs(), llvm::errs(), false, false);
    } else if (linkType == 1) {
        args.insert(args.begin(), "ld64.lld");
        lld::macho::link(args, llvm::outs(), llvm::errs(), false, false);
    } else if (linkType == 2) {
        args.insert(args.begin(), "lld-link");
        lld::coff::link(args, llvm::outs(), llvm::errs(), false, false);
    } else if (linkType == 3) {
        args.insert(args.begin(), "wasm-ld");
        lld::wasm::link(args, llvm::outs(), llvm::errs(), false, false);
    }
}

extern "C" void EXPORT LLD_LinkElf(const char** args, uint32_t arglen) {
    _LLD_Link(args, arglen, 0);
}

extern "C" void EXPORT LLD_LinkMachO(const char** args, uint32_t arglen) {
    _LLD_Link(args, arglen, 0);
}

extern "C" void EXPORT LLD_LinkCoff(const char** args, uint32_t arglen) {
    _LLD_Link(args, arglen, 0);
}

extern "C" void EXPORT LLD_LinkWasm(const char** args, uint32_t arglen) {
    _LLD_Link(args, arglen, 0);
}

extern "C" void EXPORT LLVM_InitializeAllTargetInfos(void) {
    LLVMInitializeAllTargetInfos();
}

extern "C" void EXPORT LLVM_InitializeAllTargets(void) {
    LLVMInitializeAllTargets();
}

extern "C" void EXPORT LLVM_InitializeAllTargetMCs(void) {
    LLVMInitializeAllTargetMCs();
}

extern "C" void EXPORT LLVM_InitializeAllAsmPrinters(void) {
    LLVMInitializeAllAsmPrinters();
}

extern "C" void EXPORT LLVM_InitializeAllAsmParsers(void) {
    LLVMInitializeAllAsmParsers();
}

extern "C" void EXPORT LLVM_InitializeAllDisassemblers(void) {
    LLVMInitializeAllDisassemblers();
}

/* These functions return true on failure. */
extern "C" LLVMBool EXPORT LLVM_InitializeNativeTarget(void) {
    return LLVMInitializeNativeTarget();
}

extern "C" LLVMBool EXPORT LLVM_InitializeNativeAsmParser(void) {
    return LLVMInitializeNativeAsmParser();
}

extern "C" LLVMBool EXPORT LLVM_InitializeNativeAsmPrinter(void) {
    return LLVMInitializeNativeAsmPrinter();
}

extern "C" LLVMBool EXPORT LLVM_InitializeNativeDisassembler(void) {
    return LLVMInitializeNativeDisassembler();
}
