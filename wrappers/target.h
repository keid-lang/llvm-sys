#include <llvm-c/Target.h>
#include <lld/Common/Driver.h>

#define EXPORT __attribute__((visibility("default")))

extern "C" void EXPORT LLD_LinkElf(const char** args, uint32_t arglen);
extern "C" void EXPORT LLD_LinkMachO(const char** args, uint32_t arglen);
extern "C" void EXPORT LLD_LinkCoff(const char** args, uint32_t arglen);
extern "C" void EXPORT LLD_LinkWasm(const char** args, uint32_t arglen);
extern "C" void EXPORT LLVMInitializeAllTargetInfos();
extern "C" void EXPORT LLVM_InitializeAllTargets(void);
extern "C" void EXPORT LLVM_InitializeAllTargetMCs(void);
extern "C" void EXPORT LLVM_InitializeAllAsmPrinters(void);
extern "C" void EXPORT LLVM_InitializeAllAsmParsers(void);
extern "C" void EXPORT LLVM_InitializeAllDisassemblers(void);
extern "C" LLVMBool EXPORT LLVM_InitializeNativeTarget(void);
extern "C" LLVMBool EXPORT LLVM_InitializeNativeAsmParser(void);
extern "C" LLVMBool EXPORT LLVM_InitializeNativeAsmPrinter(void);
extern "C" LLVMBool EXPORT LLVM_InitializeNativeDisassembler(void);
