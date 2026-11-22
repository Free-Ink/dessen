//! The Zig apps, for `zig build` and for zls, which reads this file to know
//! that `@import("dsn")` is the generated sdk/dessen.zig.
//!
//!   zig build                  every app, as zig-out/bin/<name>_zig.bin
//!   zig build run -Dapp=hello  one app on the desktop runner (c/b/p/n = events)
//!
//! An image is linked exactly like the C apps: sdk/crt0.S for the header and
//! entry point, sdk/app.ld for the memory layout, RV32IM with no compressed
//! instructions. `make zig` builds the same app with gcc as the linker; this
//! needs nothing but Zig (and, for `run`, the Makefile's desktop runner).
//!
//! The wrappers come from abi/freeink.api: run `python3 tools/gen_api.py` (or
//! any make target) first when the ABI changes.

const std = @import("std");

/// The Zig apps: apps/<name>.zig, each becoming <name>_zig.bin.
const apps = [_][]const u8{"hello"};

pub fn build(b: *std.Build) void {
    // The VM decodes RV32IM only: no compressed (C) instructions, no floats.
    const target = b.resolveTargetQuery(.{
        .cpu_arch = .riscv32,
        .os_tag = .freestanding,
        .abi = .none,
        .cpu_model = .{ .explicit = &std.Target.riscv.cpu.generic_rv32 },
        .cpu_features_add = std.Target.riscv.featureSet(&.{.m}),
    });
    // Small by default, as `make zig` builds it: Debug pulls in Zig's panic and
    // stack-trace machinery, well past the 64 KB an app gets.
    const optimize = b.option(std.lang.Optimize, "optimize", "Optimization mode (default: small)") orelse .small;
    const selected = b.option([]const u8, "app", "The app `zig build run` runs (default: hello)") orelse "hello";

    const dsn = b.createModule(.{
        .root_source_file = b.path("sdk/dessen.zig"),
        .target = target,
        .optimize = optimize,
        // No unwinding on the VM, and app.ld drops .eh_frame anyway.
        .unwind_tables = .none,
    });

    const run_step = b.step("run", "Run -Dapp=<name> on the desktop runner (build/dessen)");
    for (apps) |name| {
        const root = b.createModule(.{
            .root_source_file = b.path(b.fmt("apps/{s}.zig", .{name})),
            .target = target,
            .optimize = optimize,
            // No unwinding on the VM, and app.ld drops .eh_frame anyway.
            .unwind_tables = .none,
            .imports = &.{.{ .name = "dsn", .module = dsn }},
        });
        // crt0.S includes dessen_abi.h for the magic, version and exit call.
        root.addAssemblyFile(b.path("sdk/crt0.S"));
        root.addIncludePath(b.path("sdk"));

        const elf = b.addExecutable(.{ .name = b.fmt("{s}_zig", .{name}), .root_module = root });
        elf.setLinkerScript(b.path("sdk/app.ld"));
        elf.link_gc_sections = true;

        // The VM loads the flat image, header first, not the ELF.
        const image = b.addObjCopy(elf.getEmittedBin(), .{ .format = .binary });
        const image_name = b.fmt("{s}_zig.bin", .{name});
        const install = b.addInstallBinFile(image.getOutput(), image_name);
        b.getInstallStep().dependOn(&install.step);

        if (std.mem.eql(u8, name, selected)) {
            // The runner is C++ and built by the Makefile. An app's exit code
            // is its own (often the event that ended it), so it is printed
            // rather than taken as a failure.
            const runner = b.addSystemCommand(&.{ "make", "build/dessen" });
            const run = b.addSystemCommand(&.{ "sh", "-c", "build/dessen \"$0\"; echo \"exit $?\"" });
            run.addFileArg(image.getOutput());
            run.step.dependOn(&runner.step);
            run.stdio = .inherit;
            run_step.dependOn(&run.step);
        }
    }
    for (apps) |name| {
        if (std.mem.eql(u8, name, selected)) break;
    } else {
        run_step.dependOn(&b.addFail(b.fmt("no Zig app '{s}': see `apps` in build.zig", .{selected})).step);
    }
}
