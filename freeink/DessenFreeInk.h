// Runs a dessen app against the FreeInk SDK: drawing syscalls go to a
// freeink::ui::DrawTarget, events come from a freeink::ui::InputSnapshot. Any
// firmware built on FreeInkUI can host apps with this; the apps themselves
// contain no SDK code.
#pragma once
#include <FreeInkUICore.h>

#include "../vm/dessen_vm.h"

namespace dessen {

enum class State : uint8_t {
  Running, // call step() again
  Waiting, // blocked until the next input event
  Exited,
  Faulted,
};

class FreeInkHost {
 public:
  // `mem` already holds the app image in its first `len` bytes (read it from
  // storage straight into the block) and is `memCap` bytes long. The caller
  // owns it and must keep it alive while the app runs.
  bool load(uint8_t* mem, uint32_t len, uint32_t memCap);

  // Call once per UI loop tick. Runs the app until it waits for input, asks
  // for a refresh, exits, faults, or has used `budget` instructions, so a
  // runaway app cannot stall the firmware.
  //
  // The app's coordinate space is device.safeRect(): it cannot draw under a
  // bezel, and taps outside the safe area are not delivered.
  State step(freeink::ui::DrawTarget& target, const freeink::ui::DeviceContext& device,
             const freeink::ui::InputSnapshot& input, uint32_t budget = 200000);

  // Entropy for the app's random() call, e.g. esp_random. Without one the
  // app gets a weak fallback sequence, fine for tests and nothing else.
  void setRandom(uint32_t (*source)()) { random_ = source; }

  // True once after the app asked for the panel to be updated.
  bool takeRefreshRequest() {
    const bool r = refresh_;
    refresh_ = false;
    return r;
  }
  State state() const { return state_; }
  int exitCode() const { return static_cast<int>(vm_.x[10]); }
  const dvm_t& vm() const { return vm_; } // fault_addr, pc, icount for diagnostics

 private:
  static int onEcall(dvm_t* vm, void* self);
  uint32_t nextRandom();

  dvm_t vm_{};
  State state_ = State::Exited;
  bool refresh_ = false;
  uint32_t (*random_)() = nullptr;
  uint32_t fallbackSeed_ = 0x9e3779b9u;
  freeink::ui::DrawTarget* target_ = nullptr;
  freeink::ui::DeviceContext device_{};
};

} // namespace dessen
