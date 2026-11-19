// dessen VM: a sandboxed RV32IM interpreter. The guest sees one flat block of
// memory at address 0 and talks to the host only through ECALL.
#pragma once
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

typedef struct dvm {
  uint32_t x[32];      // registers; x[0] stays 0
  uint32_t pc;
  uint8_t *mem;        // guest memory, host-owned
  uint32_t mem_size;   // must be >= 4
  uint64_t icount;     // instructions executed so far
  uint32_t fault_addr; // pc or data address of the last fault
} dvm_t;

enum {
  DVM_BUDGET = 0,      // budget used up; call dvm_run again to continue
  DVM_FAULT_MEM = -1,  // fetch/load/store outside guest memory
  DVM_FAULT_INSN = -2, // illegal or unsupported instruction
};

// Called on ECALL with pc already past it, so the VM is resumable. Read args
// from vm->x[10..15] (a0-a5), the call number from vm->x[17] (a7), write the
// result to vm->x[10]. Return 0 to keep running, or a positive host-defined
// code (exit, yield, ...) to make dvm_run return it.
typedef int (*dvm_ecall_fn)(dvm_t *vm, void *ctx);

// Run up to `budget` instructions. Returns DVM_BUDGET, a DVM_FAULT_*, or the
// positive code from the ecall handler.
int dvm_run(dvm_t *vm, uint32_t budget, dvm_ecall_fn ecall, void *ctx);

// Validate an app image already read into mem[0..len) and prepare `vm` to run
// it; the rest of the block up to the size the app asked for is zeroed. `mem`
// must hold mem_cap bytes. Returns 0 if the image is invalid or wants more
// than mem_cap.
int dvm_load(dvm_t *vm, uint8_t *mem, uint32_t len, uint32_t mem_cap);

// Host-side pointer to guest [addr, addr+len), or NULL if out of range.
// Every guest pointer passed to a syscall must go through this.
void *dvm_ptr(dvm_t *vm, uint32_t addr, uint32_t len);

#ifdef __cplusplus
}
#endif
