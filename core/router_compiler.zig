const std = @import("std");

pub const FileExtension = enum {
    javascript,
    typescript,
    mustache,

    pub const Map = std.ComptimeStringMap(FileExtension, .{
        .{ ".js", .javascript },
        .{ ".ts", .typescript },
        .{ ".mustache", .mustache },
    });
};

pub const RouterCompiler = struct {
    pages_dir: std.fs.Dir,
    allocator: std.mem.Allocator,

    /// Walks the directory and compiles the routes into the target file.
    pub fn walkAndCompile(compiler: *RouterCompiler, writer: anytype) !void {
        var walker = try compiler.pages_dir.walk(compiler.allocator);
        defer walker.deinit();

        try writer.writeAll("/** @param {string} route @returns {Response} */export function handle(route){let [pathStr,query]=route.split('?'),pathParts=route.split('/');");

        while (try walker.next()) |entry| {
            if (entry.kind == .File) {
                var ext = FileExtension.Map.get(std.fs.path.extension(entry.basename)) orelse continue;
                _ = ext;
                var pages_path = try std.fs.path.resolve(compiler.allocator, &.{"pages"});
                defer compiler.allocator.free(pages_path);

                try writer.writeAll("if(");

                var start: usize = 0;
                var part: usize = 0;
                for (entry.path) |c, i| {
                    if (c == '/') {
                        start = i + 1;
                        part += 1;
                        try writer.print("pathParts[{}]=='{s}'", .{ part, entry.path[start..i] });
                        continue;
                    }
                }

                try writer.writeAll("){}");
            }
        }
        try writer.writeAll("}");
    }
};
