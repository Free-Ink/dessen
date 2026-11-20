// A dessen app written in Zig — proof the RV32IM runtime is language-agnostic.
// The ecall wrappers (dsn.clear, dsn.text_box, …) are generated from
// abi/freeink.api by tools/gen_api.py, same as the C header. This file links
// against the same sdk/crt0.S + sdk/app.ld as the C apps. Freestanding, no std.
//
//   zig build-obj apps/hello.zig -target riscv32-freestanding-none \
//       -mcpu=generic_rv32+m -OReleaseSmall -femit-bin=build/hello_zig.o
//   (then link with gcc: see `make zig`)

const dsn = @import("dsn"); // generated ecall wrappers + numbers (see `make zig`)

fn text(x: i32, y: i32, s: []const u8) void {
    _ = dsn.text_box(x, y, 0, 0, s, 0, 0, 1, 0); // w=h=0, FONT_SMALL, ALIGN_LEFT, 1 line
}

// crt0.S calls main, then exit(a7)'s with main's return value as the exit code.
export fn main() i32 {
    dsn.clear();
    text(40, 60, "Hello from Zig on dessen");
    text(40, 100, "Press any button");
    dsn.refresh();
    return @bitCast(dsn.wait_event()); // exit code = the button pressed
}
