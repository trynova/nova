const std = @import("std");
const mustache = @import("parsers/mustache.zig");
const css_encoder = @import("utils/css.zig");
const AppConfig = @import("config.zig").AppConfig;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    var arena = std.heap.ArenaAllocator.init(gpa.allocator());
    defer arena.deinit();
    var allocator = arena.allocator();

    var config = try AppConfig.init();
    defer config.dirs.closeAll();

    const start = try std.time.Instant.now();
    const file_path = "template.mustache";

    const input = blk: {
        var f = try config.dirs.cwd.openFile(file_path, .{});
        defer f.close();
        break :blk try f.readToEndAlloc(allocator, 4294967296);
    };

    var stream = mustache.TokenStream{ .iter = .{ .bytes = input, .i = 0 } };

    var tokens = std.MultiArrayList(mustache.Token){};
    try tokens.ensureTotalCapacity(allocator, input.len / 32); // On average probably true

    while (true) {
        const tok = stream.next();
        if (tok.kind == .eof) break;
        try tokens.append(allocator, tok);
    }

    var parser = mustache.Parser{
        .index = 0,
        .buffer = input,
        .token_spans = tokens.items(.span),
        .token_kinds = tokens.items(.kind),
        .errors = std.ArrayList(mustache.Parser.Error).init(allocator),
        .nodes = blk: {
            var nodes = std.MultiArrayList(mustache.Node){};
            try nodes.ensureTotalCapacity(allocator, tokens.len / 4);
            break :blk nodes;
        },
        .allocator = allocator,
        .tag_stack = std.ArrayList(usize).init(allocator),
    };

    parser.parse() catch {
        var stdout = std.io.getStdOut().writer();
        var tty = std.debug.detectTTYConfig();
        var newLineCount: u32 = 0;
        var bufIdx: usize = 0;
        var lastNewLine: usize = 0;
        for (parser.errors.items) |err| {
            if (err.kind.isNote()) { // notes can be backtraced :(
                bufIdx = 0;
                newLineCount = 0;
                lastNewLine = 0;
            }
            var tokBufIdx = parser.token_spans[err.index].start;
            while (bufIdx < tokBufIdx) : (bufIdx += 1) {
                if (input[bufIdx] == '\n') {
                    newLineCount += 1;
                    lastNewLine = bufIdx;
                }
            }
            tty.setColor(stdout, .Bold);
            try stdout.print("{s}:{}:{}: ", .{ file_path, newLineCount + 1, tokBufIdx - lastNewLine });
            tty.setColor(stdout, .Reset);

            tty.setColor(stdout, @as(std.debug.TTY.Color, if (err.kind.isNote()) .Cyan else .Red));
            var flag = if (err.kind.isNote()) "note" else "error";
            try stdout.print("{s}: ", .{flag});
            tty.setColor(stdout, .Reset);

            tty.setColor(stdout, .Bold);
            try err.write(&parser, stdout);
            tty.setColor(stdout, .Reset);
            try stdout.writeAll("\n");
        }
        return;
    };

    var out_file_path = blk: {
        var old = try std.ArrayList(u8).initCapacity(allocator, file_path.len + 3);
        try old.appendSlice(file_path);
        try old.appendSlice(".js");
        break :blk old.toOwnedSlice();
    };
    var ofile = try config.dirs.dist.createFile(out_file_path, .{});
    try ofile.lock(.Exclusive);
    errdefer ofile.unlock();
    // var sourceMapFile = try std.io.BufferedAtomicFile.create(allocator, resourcesDir, "template.mustache.js", .{});
    // defer sourceMapFile.destroy();

    var ofile_buf = std.io.bufferedWriter(ofile.writer());
    var ofile_writer = ofile_buf.writer();

    try mustache.Compiler(.{
        .include_source_map = true,
    }).compile(
        ofile_writer,
        &parser,
        null,
    );

    try ofile_buf.flush();
    ofile.unlock();

    std.debug.print("Successfully compiled file in {d}ms\n", .{@intToFloat(f64, (try std.time.Instant.now()).since(start)) / 1_000_000.0});
}
