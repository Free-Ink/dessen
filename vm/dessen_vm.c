#include "dessen_vm.h"

#include <string.h>

#include "../sdk/dessen_abi.h"

int dvm_load(dvm_t *vm, uint8_t *mem, uint32_t len, uint32_t mem_cap) {
  uint32_t h[4]; // magic, mem_size, entry, api_version
  if (len < DSN_HEADER_SIZE || len > mem_cap) return 0;
  memcpy(h, mem, sizeof h);
  if (h[0] != DSN_MAGIC || h[3] != DSN_API_VERSION) return 0;
  if (h[1] < len || h[1] > mem_cap || (h[1] & 15) || h[2] >= len || (h[2] & 3)) return 0;
  memset(mem + len, 0, h[1] - len);
  memset(vm, 0, sizeof *vm);
  vm->mem = mem;
  vm->mem_size = h[1];
  vm->pc = h[2];
  vm->x[2] = h[1]; // sp = top of memory
  return 1;
}

// ponytail: RV32IM only, no compressed (C) instructions. Apps are ~25% larger
// than they could be; add a 16-bit decoder if app size starts to matter.
// ponytail: assumes a little-endian host (ESP32, x86, arm64 all are).

void *dvm_ptr(dvm_t *vm, uint32_t addr, uint32_t len) {
  if (len > vm->mem_size || addr > vm->mem_size - len) return 0;
  return vm->mem + addr;
}

int dvm_run(dvm_t *vm, uint32_t budget, dvm_ecall_fn ecall, void *ctx) {
  uint32_t *x = vm->x, pc = vm->pc, n = 0;
  uint8_t *mem = vm->mem;
  const uint32_t msz = vm->mem_size;
  int rc = DVM_BUDGET;

#define FAULT(code, addr) \
  do { vm->fault_addr = (addr); rc = (code); goto out; } while (0)
#define SEXT(v, shift) ((uint32_t)((int32_t)(v) >> (shift)))

  for (; n < budget; n++) {
    if ((pc & 3) || pc > msz - 4) FAULT(DVM_FAULT_MEM, pc);
    uint32_t in;
    memcpy(&in, mem + pc, 4);
    const uint32_t rd = (in >> 7) & 31, f3 = (in >> 12) & 7, rs2 = (in >> 20) & 31, f7 = in >> 25;
    const uint32_t a = x[(in >> 15) & 31], b = x[rs2];
    const uint32_t immI = SEXT(in, 20);
    uint32_t npc = pc + 4, v = 0;
    int wr = 1;  // write v to rd

    switch (in & 0x7f) {
      case 0x37: v = in & 0xfffff000; break;        // lui
      case 0x17: v = pc + (in & 0xfffff000); break; // auipc
      case 0x6f:                                    // jal
        v = npc;
        npc = pc + ((SEXT(in, 31) << 20) | (in & 0xff000) | ((in >> 9) & 0x800) | ((in >> 20) & 0x7fe));
        break;
      case 0x67: // jalr
        if (f3) FAULT(DVM_FAULT_INSN, pc);
        v = npc;
        npc = (a + immI) & ~1u;
        break;
      case 0x63: { // branches
        int take;
        switch (f3) {
          case 0: take = a == b; break;
          case 1: take = a != b; break;
          case 4: take = (int32_t)a < (int32_t)b; break;
          case 5: take = (int32_t)a >= (int32_t)b; break;
          case 6: take = a < b; break;
          case 7: take = a >= b; break;
          default: FAULT(DVM_FAULT_INSN, pc);
        }
        if (take) npc = pc + ((SEXT(in, 31) << 12) | ((in << 4) & 0x800) | ((in >> 20) & 0x7e0) | ((in >> 7) & 0x1e));
        wr = 0;
        break;
      }
      case 0x03: { // loads
        const uint32_t addr = a + immI, size = 1u << (f3 & 3);
        if (f3 == 3 || f3 > 5) FAULT(DVM_FAULT_INSN, pc);
        if (addr > msz - size) FAULT(DVM_FAULT_MEM, addr);
        memcpy(&v, mem + addr, size);
        if (f3 == 0) v = (uint32_t)(int8_t)v;
        if (f3 == 1) v = (uint32_t)(int16_t)v;
        break;
      }
      case 0x23: { // stores
        const uint32_t addr = a + ((SEXT(in, 25) << 5) | rd), size = 1u << f3;
        if (f3 > 2) FAULT(DVM_FAULT_INSN, pc);
        if (addr > msz - size) FAULT(DVM_FAULT_MEM, addr);
        memcpy(mem + addr, &b, size);
        wr = 0;
        break;
      }
      case 0x13: // op-imm
        switch (f3) {
          case 0: v = a + immI; break;
          case 1:
            if (f7) FAULT(DVM_FAULT_INSN, pc);
            v = a << rs2;
            break;
          case 2: v = (int32_t)a < (int32_t)immI; break;
          case 3: v = a < immI; break;
          case 4: v = a ^ immI; break;
          case 5:
            if (f7 & ~0x20u) FAULT(DVM_FAULT_INSN, pc);
            v = f7 ? SEXT(a, rs2) : a >> rs2;
            break;
          case 6: v = a | immI; break;
          case 7: v = a & immI; break;
        }
        break;
      case 0x33: // op
        if (f7 == 1) { // M extension
          const int32_t sa = (int32_t)a, sb = (int32_t)b;
          const int ovf = a == 0x80000000u && b == 0xffffffffu;
          switch (f3) {
            case 0: v = a * b; break;
            case 1: v = (uint32_t)(((int64_t)sa * (int64_t)sb) >> 32); break;
            case 2: v = (uint32_t)(((int64_t)sa * (int64_t)(uint64_t)b) >> 32); break;
            case 3: v = (uint32_t)(((uint64_t)a * (uint64_t)b) >> 32); break;
            case 4: v = b == 0 ? 0xffffffffu : ovf ? a : (uint32_t)(sa / sb); break;
            case 5: v = b == 0 ? 0xffffffffu : a / b; break;
            case 6: v = b == 0 ? a : ovf ? 0 : (uint32_t)(sa % sb); break;
            case 7: v = b == 0 ? a : a % b; break;
          }
        } else if (f7 == 0 || (f7 == 0x20 && (f3 == 0 || f3 == 5))) {
          const uint32_t sh = b & 31;
          switch (f3) {
            case 0: v = f7 ? a - b : a + b; break;
            case 1: v = a << sh; break;
            case 2: v = (int32_t)a < (int32_t)b; break;
            case 3: v = a < b; break;
            case 4: v = a ^ b; break;
            case 5: v = f7 ? SEXT(a, sh) : a >> sh; break;
            case 6: v = a | b; break;
            case 7: v = a & b; break;
          }
        } else {
          FAULT(DVM_FAULT_INSN, pc);
        }
        break;
      case 0x0f: wr = 0; break; // fence: nop
      case 0x73: {              // ecall only; ebreak and CSRs are illegal
        if (in != 0x73) FAULT(DVM_FAULT_INSN, pc);
        vm->pc = npc;
        const int r = ecall(vm, ctx);
        x[0] = 0;
        if (r) {
          n++;
          pc = npc;
          rc = r;
          goto out;
        }
        wr = 0;
        break;
      }
      default: FAULT(DVM_FAULT_INSN, pc);
    }
    if (wr && rd) x[rd] = v;
    pc = npc;
  }
out:
  vm->pc = pc;
  vm->icount += n;
  return rc;
}
