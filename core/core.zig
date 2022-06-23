const std = @import("std");
const cosmo = @import("cosmo");
const css_encoder = @import("utils/css.zig");
const AppConfig = @import("config.zig").AppConfig;
const RouterCompiler = @import("router_compiler.zig").RouterCompiler;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    var gpa_allocator = gpa.allocator();
    var arena = std.heap.ArenaAllocator.init(gpa_allocator);
    defer arena.deinit();
    var allocator = arena.allocator();
    _ = allocator;

    var config = try AppConfig.init();
    defer config.dirs.closeAll();

    {
        // Compiles the router for all of the different files.
        var router_compiler = RouterCompiler{ .pages_dir = config.dirs.pages, .allocator = gpa_allocator };

        var router_file = config.dirs.dist.openFile("_router.js", .{ .mode = .write_only }) catch |e|
            if (e == error.FileNotFound) try config.dirs.dist.createFile("_router.js", .{}) else return e;
        defer router_file.close();

        var router_file_buffer = std.io.bufferedWriter(router_file.writer());
        try router_compiler.walkAndCompile(router_file_buffer.writer());
        try router_file_buffer.flush();
    }
}
