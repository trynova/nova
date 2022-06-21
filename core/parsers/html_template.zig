const std = @import("std");

pub const Token = struct {
    start: usize,
    kind: Kind,

    pub const Kind = enum {
        boring_js,
        boring_html,
        /// This could be anywhere, really.
        whitespace,
        /// <
        tag_start,
        /// </
        tag_close_start,
        tag_name,
        /// >
        tag_end,
        /// />
        tag_self_close_end,

        tag_attr_name,
        /// =
        tag_attr_equal,

        tag_attr_value_raw,
        tag_attr_value_single_quote,
        tag_attr_value_double_quote,
        bogus_tag_end,

        /// A string that was started, but not ended.
        invalid_string_fragment,

        tag_whitespace,

        eof,

        pub fn symbol(kind: Kind) []const u8 {
            return switch (kind) {
                .boring_js => "some JavaScript",
                .boring_html => "some HTML content",
                .whitespace, .tag_whitespace => "some whitespace",
                .tag_start => "a tag start '<'",
                .tag_close_start => "a closing tag start '</'",
                .tag_name => "a tag name",
                .tag_end => "a tag end '>'",
                .tag_attr_name => "a tag attribute name",
                .tag_attr_equal => "an equal sign '=' after the attribute name",
                .tag_attr_value_raw => "a raw tag attribute value",
                .tag_attr_value_single_quote => "a single-quoted tag attribute value",
                .tag_attr_value_double_quote => "a double-quoted tag attribute value",
                .bogus_tag_end => "a bogus tag end",
                .invalid_string_fragment => "an invalid string fragment",
                .eof => "the end of input",
            };
        }
    };
};

pub const TokenStream = struct {
    state: State = .html,
    iter: std.unicode.Utf8Iterator,

    pub const State = enum {
        html,
        // tag_start
        mebbe_tag_close_start_or_comment,
        html_whitespace,
        html_content,
        // html_comment_bang,
        // html_comment_dash1,
        // html_comment_dash2,
        tag_name,
        /// Whitespace, but ya know, in the tags.
        tag_whitespace,
        tag_attr_name,
        tag_attr_after_name_whitespace,
        tag_attr_equal,
        tag_attr_after_equal,
        tag_attr_after_equal_whitespace,
        tag_attr_value_double_quote,
        tag_attr_value_double_quote_esc,
        tag_attr_value_single_quote,
        tag_attr_value_single_quote_esc,
        tag_attr_value_raw,
        tag_end,
        bogus_tag_end,
        eof,
    };

    fn nextCodepoint(stream: *TokenStream) u21 {
        return stream.iter.nextCodepoint() orelse 0;
    }

    inline fn recall(stream: *TokenStream, n: anytype) void {
        stream.iter.i -= n;
    }

    inline fn totalRecall(stream: *TokenStream, cp: u21) void {
        stream.iter.i -= std.unicode.utf8CodepointSequenceLength(cp) catch unreachable;
    }

    pub fn next(stream: *TokenStream) Token {
        var start = stream.iter.i;
        var cur = stream.nextCodepoint();

        var kind: Token.Kind = undefined;
        var flags: packed struct { html_content_single_whitespace: bool = false } = .{};

        loop: while (true) : (cur = stream.nextCodepoint()) {
            switch (stream.state) {
                .html => switch (cur) {
                    '<' => stream.state = .mebbe_tag_close_start_or_comment,
                    ' ', '\t', '\r', '\n' => stream.state = .html_whitespace,
                    0 => {
                        stream.recall(1);
                        stream.state = .eof;
                    },
                    else => |cp| {
                        stream.totalRecall(cp);
                        stream.state = .html_content;
                    },
                },
                .html_content => switch (cur) {
                    ' ', '\t', '\r', '\n' => {
                        if (flags.html_content_single_whitespace) {
                            stream.recall(2); // we must account for 2 whitespace chars
                            kind = .boring_html;
                            stream.state = .html_whitespace;
                            break :loop;
                        }

                        flags.html_content_single_whitespace = true;
                    },
                    0 => {
                        stream.recall(1);
                        stream.state = .eof;
                        kind = .boring_html;
                        break :loop;
                    },
                    '<' => {
                        stream.state = .mebbe_tag_close_start_or_comment;
                        kind = .whitespace;
                        break :loop;
                    },
                    else => {
                        if (flags.html_content_single_whitespace) flags.html_content_single_whitespace = false;
                    },
                },
                .html_whitespace => switch (cur) {
                    ' ', '\t', '\r', '\n' => {},
                    0 => {
                        stream.recall(1);
                        stream.state = .eof;
                        kind = .whitespace;
                        break :loop;
                    },
                    else => |cp| {
                        stream.totalRecall(cp);
                        stream.state = .html;
                        kind = .whitespace;
                        break :loop;
                    },
                },
                .mebbe_tag_close_start_or_comment => switch (cur) {
                    ' ', '\t', '\r', '\n' => {
                        // sike, it's a jumpsuit
                        stream.state = .html_content;
                    },
                    '/' => {
                        kind = .tag_close_start;
                        stream.state = .tag_name;
                        break :loop;
                    },
                    0 => {
                        stream.state = .eof;
                        kind = .boring_html;
                        break :loop;
                    },
                    '!' => @panic("todo: handle comments"),
                    else => {
                        stream.iter.i = start + 1; // we know < is one u8
                        kind = .tag_start;
                        stream.state = .tag_name;
                        break :loop;
                    },
                },
                .tag_name => switch (cur) {
                    ' ', '\t', '\r', '\n' => {
                        stream.recall(1);
                        kind = .tag_name;
                        stream.state = .tag_whitespace;
                        break :loop;
                    },
                    0 => {
                        stream.state = .bogus_tag_end;
                        kind = .tag_name;
                        break :loop;
                    },
                    '>', '/' => {
                        stream.recall(1);
                        kind = .tag_name;
                        stream.state = .tag_end;
                        break :loop;
                    },
                    else => {},
                },
                .tag_whitespace => switch (cur) {
                    ' ', '\t', '\r', '\n' => {},

                    '>', '/' => {
                        stream.recall(1);
                        kind = .whitespace;
                        stream.state = .tag_end;
                        break :loop;
                    },
                    0 => {
                        stream.state = .bogus_tag_end;
                        kind = .whitespace;
                        break :loop;
                    },
                    else => |cp| {
                        stream.totalRecall(cp);
                        kind = .whitespace;
                        stream.state = .tag_attr_name;
                        break :loop;
                    },
                },
                .tag_attr_name => switch (cur) {
                    ' ', '\t', '\r', '\n' => {
                        stream.recall(1);
                        kind = .tag_attr_name;
                        stream.state = .tag_attr_after_name_whitespace;
                        break :loop;
                    },
                    '=' => {
                        stream.recall(1);
                        kind = .tag_attr_name;
                        stream.state = .tag_attr_equal;
                        break :loop;
                    },
                    0 => {
                        kind = .tag_attr_name;
                        stream.state = .bogus_tag_end;
                        break :loop;
                    },
                    '>', '/' => {
                        stream.recall(1);
                        kind = .tag_attr_name;
                        stream.state = .tag_end;
                        break :loop;
                    },
                    else => {},
                },
                .tag_attr_after_name_whitespace => switch (cur) {
                    ' ', '\t', '\r', '\n' => {},
                    '>', '/' => {
                        stream.recall(1);
                        kind = .whitespace;
                        stream.state = .tag_end;
                        break :loop;
                    },
                    '=' => {
                        stream.recall(1);
                        kind = .whitespace;
                        stream.state = .tag_attr_equal;
                        break :loop;
                    },
                    0 => {
                        kind = .whitespace;
                        stream.state = .bogus_tag_end;
                        break :loop;
                    },
                    else => |cp| {
                        stream.totalRecall(cp);
                        kind = .whitespace;
                        stream.state = .tag_attr_name;
                        break :loop;
                    },
                },
                .tag_attr_equal => {
                    kind = .tag_attr_equal;
                    stream.state = .tag_attr_after_equal;
                    break :loop;
                },
                .tag_attr_after_equal => switch (cur) {
                    ' ', '\t', '\r', '\n' => stream.state = .tag_attr_after_equal_whitespace,
                    '>', '/' => stream.state = .tag_end,
                    '\'' => stream.state = .tag_attr_value_single_quote,
                    '"' => stream.state = .tag_attr_value_double_quote,
                    0 => {
                        stream.state = .eof;
                        kind = .bogus_tag_end;
                        break :loop;
                    },
                    else => stream.state = .tag_attr_value_raw,
                },
                .tag_attr_after_equal_whitespace => switch (cur) {
                    ' ', '\t', '\r', '\n' => {},
                    '\'' => {
                        stream.recall(1);
                        stream.state = .tag_attr_value_single_quote;
                        kind = .whitespace;
                        break :loop;
                    },
                    '\"' => {
                        stream.recall(1);
                        stream.state = .tag_attr_value_double_quote;
                        kind = .whitespace;
                        break :loop;
                    },
                    '>', '/' => {
                        stream.recall(1);
                        stream.state = .tag_end;
                        kind = .whitespace;
                        break :loop;
                    },
                    0 => {
                        stream.state = .bogus_tag_end;
                        kind = .whitespace;
                        break :loop;
                    },
                    else => |cp| {
                        stream.totalRecall(cp);
                        stream.state = .tag_attr_value_raw;
                        kind = .whitespace;
                        break :loop;
                    },
                },
                .tag_attr_value_single_quote => switch (cur) {
                    '\\' => stream.state = .tag_attr_value_single_quote_esc,
                    '\'' => {
                        stream.state = .tag_whitespace;
                        kind = .tag_attr_value_single_quote;
                        break :loop;
                    },
                    0 => {
                        stream.recall(1);
                        stream.state = .eof;
                        kind = .invalid_string_fragment;
                        break :loop;
                    },
                    else => {},
                },
                .tag_attr_value_single_quote_esc => switch (cur) {
                    '\'' => stream.state = .tag_attr_value_single_quote,
                    0 => {
                        stream.recall(1);
                        stream.state = .eof;
                        kind = .invalid_string_fragment;
                        break :loop;
                    },
                    else => {},
                },
                .tag_attr_value_double_quote => switch (cur) {
                    '\\' => stream.state = .tag_attr_value_double_quote_esc,
                    '"' => {
                        stream.state = .tag_whitespace;
                        kind = .tag_attr_value_double_quote;
                        break :loop;
                    },
                    0 => {
                        stream.recall(1);
                        stream.state = .eof;
                        kind = .invalid_string_fragment;
                        break :loop;
                    },
                    else => {},
                },
                .tag_attr_value_double_quote_esc => switch (cur) {
                    '"' => stream.state = .tag_attr_value_double_quote,
                    0 => {
                        stream.recall(1);
                        stream.state = .eof;
                        kind = .invalid_string_fragment;
                        break :loop;
                    },
                    else => {},
                },
                .tag_attr_value_raw => switch (cur) {
                    ' ', '\t', '\r', '\n' => {
                        stream.recall(1);
                        stream.state = .tag_whitespace;
                        kind = .tag_attr_value_raw;
                        break :loop;
                    },
                    '>', '/' => {
                        stream.recall(1);
                        stream.state = .tag_end;
                        kind = .tag_attr_value_raw;
                        break :loop;
                    },
                    0 => {
                        kind = .tag_attr_value_raw;
                        stream.state = .bogus_tag_end;
                        break :loop;
                    },
                    else => {},
                },
                .tag_end => switch (cur) {
                    '/' => switch (stream.next()) {
                        '>' => {
                            stream.state = .html;
                            kind = .tag_self_close_end;
                            break :loop;
                        },
                        else => @panic("todo"),
                    },
                    '>' => {
                        stream.state = .html;
                        kind = .tag_end;
                        break :loop;
                    },
                    else => unreachable,
                },
                .bogus_tag_end => {
                    kind = .bogus_tag_end;
                    stream.state = .eof;
                    break :loop;
                },
                .eof => {
                    kind = .eof;
                    break :loop;
                },
            }
        }

        return Token{
            .kind = kind,
            .start = start,
        };
    }
};

pub const Node = struct {
    kind: Node.Kind,
    /// The index of the main token.
    main_token: usize,
    sides: Node.Sides,

    pub const Sides = struct {
        /// The left hand side.
        lhs: usize,
        /// the right hand side.
        rhs: usize,
    };

    pub const Kind = enum {
        tag_start,
        tag_self_closing,
        tag_end,
    };

    pub const Null = 4294967296;

    // pub fn isClosingTag(node: *const Node) bool {}
};

pub const Diagnostic = struct {
    /// The index of the main token associated with the diagnostic.
    main_token: usize,
    kind: Kind,
    data: union {
        expected_token: struct { expected: Token.Kind, got: Token.Kind },
        expected_matching_token_slice: struct { schema: []const Token.Kind, got: Token.Kind },
        tag_open_here_index: usize,
        mismatched_closing_expected_tag_name: []const u8,
    } = undefined,

    pub const Level = enum {
        @"error",
        warning,
        note,
    };

    /// Writes the diagnostic but does not include the log level for ease of
    /// custom formatting.
    pub fn write(diag: *const Diagnostic, writer: anytype) !void {
        switch (diag.kind) {
            .expected_token => {
                const data = diag.data.expected_token;
                writer.print("expected {} but got {}", data.expected.symbol(), data.got.symbol());
            },
            .expected_matching_token => {
                const data = diag.data.expected_matching_token_slice;
                writer.writeAll("expected ");

                switch (data.schema.len) {
                    0 => writer.writeAll("???"),
                    1 => writer.writeAll(data.schema[0], data.schema[0].symbol()),
                    2 => {
                        writer.writeAll(data.schema[0].symbol());
                        writer.writeAll(" or ");
                        writer.writeAll(data.schema[1].symbol());
                    },
                    else => |n| {
                        var i = 0;
                        while (i < n) : (i += 1) {
                            if (i + 1 == n)
                                writer.writeAll(" , or ")
                            else if (i != 0)
                                writer.writeAll(" , ");

                            writer.writeAll(data.schema[i].symbol());
                        }
                    },
                }

                writer.print(" but got {}", data.got.symbol());
            },
            else => @panic("todo"),
        }
    }

    pub const Kind = enum {
        // errors
        expected_token,
        expected_matching_token,
        mismatched_closing_tag,

        // warnings

        // notes
        tag_open_here,

        pub fn level(kind: Kind) Level {
            return switch (kind) {
                .tag_open_here => .note,
                else => .@"error",
            };
        }
    };
};

pub const Parser = struct {
    /// The current token index.
    index: usize,
    token_starts: []const usize,
    token_kinds: []const Token.Kind,
    diagnostics: std.ArrayList(Diagnostic),
    allocator: std.mem.Allocator,
    failed: bool = false,
    nodes: std.MultiArrayList(Node),
    tag_open_count: std.StringHashMap(u32),
    logging: bool = true,
    buffer: []const u8,

    pub const E = error{
        ParseError,
    };

    pub fn deinit(p: *Parser) void {
        p.diagnostics.deinit();
        p.tag_open_count.deinit();
    }

    inline fn addDiagnostic(p: *Parser, diag: Diagnostic) !anyerror {
        if (p.logging == false) return error.ParseError;
        if (!p.failed and diag.kind.level() == .@"error") p.failed = true;
        try p.diagnostics.append(diag);
        return error.ParseError;
    }

    inline fn addNode(p: *Parser, node: Node) !void {
        try p.nodes.append(p.allocator, node);
    }

    inline fn expect(p: *Parser, kind: Token.Kind) !usize {
        var toki = p.nextToken();

        if (toki == null or p.token_kinds[toki] != kind) {
            p.index -= 1;
            return try p.addDiagnostic(.{
                .kind = .expected_token,
                .data = .{ .expected_token_kind = kind },
            });
        }

        return toki;
    }

    inline fn eat(p: *Parser, kind: Token.Kind) ?usize {
        var toki = p.nextToken();

        if (toki == null or p.token_kinds[toki] != kind) {
            p.index -= 1;
            return null;
        }

        return toki;
    }

    fn nextToken(p: *Parser) ?usize {
        if (p.index + 1 == p.token_starts.len) {
            return null;
        } else {
            p.index += 1;
            return p.index - 1;
        }
    }

    fn peekToken(p: *Parser) ?usize {
        return if (p.index + 1 == p.token_starts.len)
            null
        else
            p.index - 1;
    }

    inline fn eatWhitespace(p: *Parser) void {
        while (p.index < p.token_kinds.len) {
            var cur = p.token_kinds[p.index];
            if (cur != .whitespace) break;
        }
    }

    fn parseAttr(_: *Parser) void {}

    fn getTokenContent(p: *const Parser, toki: usize) []const u8 {
        return p.buffer[p.token_starts[toki]..p.token_starts[toki + 1]];
    }

    fn parseOpenTag(p: *Parser) !void {
        // ASSUME DONE: try p.eat(.tag_start);
        const namei = try p.expect(.tag_name);
        var start_node_index = try p.addNode(undefined);
        p.eatWhitespace();

        while (true) {
            p.eat(.tag_attr_name) orelse break;
            p.eatWhitespace();
            _ = p.eat(.tag_attr_equal) orelse {
                p.eatWhitespace();
                continue;
            };
            p.eatWhitespace();

            const maybeValueI = p.peekToken() orelse break;
            switch (p.token_kinds[maybeValueI]) {
                .tag_attr_value_raw, .tag_attr_value_double_quote, .tag_attr_value_single_quote => p.index += 1,
                .tag_end, .bogus_tag_end => break,
                else => unreachable,
            }
            p.eatWhitespace();
        }

        const toki = p.nextToken() orelse unreachable;

        var entry = try p.tag_open_count.getOrPutValue(p.getTokenContent(namei), 0);
        entry.value_ptr.* += 1;

        p.nodes[start_node_index] = Node{
            .main_token = namei - 1,
            .data = .{
                .lhs = start_node_index + 1,
                .rhs = toki,
            },
        };
    }

    fn parseCloseTag(_: *Parser) void {
        // ASSUME DONE: try p.eat(.tag_close_start);
    }

    pub fn parse(_: *Parser) void {}
};

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    var arena = std.heap.ArenaAllocator.init(gpa.allocator());
    defer arena.deinit();

    var allocator = arena.allocator();

    var stream = TokenStream{
        .iter = .{
            .i = 0,
            .bytes = "  </hey a=hey>",
        },
    };

    var tokens = std.MultiArrayList(Token){};

    while (true) {
        const tok = stream.next();
        try tokens.append(allocator, tok);
        if (tok.kind == .eof) break;
    }

    var parser = Parser{
        .index = 0,
        .token_starts = tokens.items(.start),
        .token_kinds = tokens.items(.kind),
        .diagnostics = std.ArrayList(Diagnostic).init(allocator),
        .tag_open_count = std.AutoHashMap([]const u8, u32).init(allocator),
        .allocator = allocator,
    };

    parser.parse();
}
