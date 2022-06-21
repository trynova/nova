const std = @import("std");

pub const Token = struct {
    kind: Kind,
    span: Span,

    pub const Span = struct { start: usize, end: usize };

    pub const Kind = enum {
        eof,
        content,

        var_name,

        /// {{
        var_tag_start,
        /// {{{ OR {{&
        raw_var_tag_start,
        /// {{#
        section_tag_start,
        /// {{/
        section_tag_end_start,
        /// {{^
        inv_section_tag_start,
        /// {{>
        partial_tag_start,
        /// }}
        tag_end,

        unexpected_symbol,
        nonterminated_comment,

        pub fn symbol(kind: Kind) []const u8 {
            return switch (kind) {
                .eof => "the end of input",
                .content => "some content",
                .var_name => "a variable name",
                .var_tag_start => "a variable tag start '{{'",
                .raw_var_tag_start => "a raw variable tag start '{{{' or '{{&'",
                .section_tag_start => "a section tag start '{{#'",
                .section_tag_end_start => "a section tag end start '{{/'",
                .inv_section_tag_start => "an inverted section tag start '{{^'",
                .partial_tag_start => "a partial tag start '{{>'",
                .tag_end => "a tag end '}}'",
                .unexpected_symbol => "an unexpected symbol",
                .nonterminated_comment => "a non-terminated comment",
            };
        }
    };
};

// I don't really like how organized the states are within templates. I'd rather
// have it just parse everything inside templates as a stream of tokens and the
// parse can then validate it. This would allow us to have better error messages
// when the template innards are invalid.

pub const TokenStream = struct {
    iter: std.unicode.Utf8Iterator,
    state: State = .content,
    has_partial: bool = false,

    const State = enum {
        content,
        var_name,
        brace1,
        brace2,
        closebrace1,
        unexpected_symbol,
    };

    inline fn nextCodepoint(stream: *TokenStream) u21 {
        return stream.iter.nextCodepoint() orelse 0;
    }

    inline fn isWhitespace(cp: u21) bool {
        return switch (cp) {
            ' ', '\t', '\n', '\r' => true,
            else => false,
        };
    }

    pub fn next(stream: *TokenStream) Token {
        var start = stream.iter.i;
        var cur = stream.nextCodepoint();

        if (cur == 0) {
            return Token{
                .kind = .eof,
                .span = .{ .start = start, .end = start },
            };
        }

        switch (stream.state) {
            // Only disallow *starting* states here
            .content, .unexpected_symbol, .brace2, .closebrace1 => {},
            else => {
                while (isWhitespace(cur)) : (cur = stream.nextCodepoint()) {}
                start = stream.iter.i;
            },
        }

        while (true) : (cur = stream.nextCodepoint()) {
            switch (stream.state) {
                .content => switch (cur) {
                    '{' => stream.state = .brace1,
                    0 => return Token{ .kind = if (start != stream.iter.i) .content else .eof, .span = .{ .start = start, .end = stream.iter.bytes.len } },
                    else => {},
                },
                .brace1 => switch (cur) {
                    '{' => {
                        stream.state = .brace2;
                        if (start != stream.iter.i - 2) {
                            return Token{
                                .kind = .content,
                                .span = .{ .start = start, .end = stream.iter.i - 2 },
                            };
                        } else {
                            // This fixes an overflow so it probably just needed
                            // to offset the start because we were skipping the
                            // first content area.
                            start = stream.iter.i;
                        }
                    },
                    else => stream.state = .content,
                },
                .brace2 => {
                    while (isWhitespace(cur)) : (cur = stream.nextCodepoint()) {}

                    switch (cur) {
                        '#' => {
                            stream.state = .var_name;
                            return Token{
                                .kind = .section_tag_start,
                                .span = .{ .start = start - 2, .end = stream.iter.i },
                            };
                        },
                        '/' => {
                            stream.state = .var_name;
                            return Token{
                                .kind = .section_tag_end_start,
                                .span = .{ .start = start - 2, .end = stream.iter.i },
                            };
                        },
                        '^' => {
                            stream.state = .var_name;
                            return Token{
                                .kind = .inv_section_tag_start,
                                .span = .{ .start = start - 2, .end = stream.iter.i },
                            };
                        },
                        '>' => {
                            stream.state = .var_name;
                            if (!stream.has_partial) {
                                stream.has_partial = true;
                            }
                            return Token{
                                .kind = .partial_tag_start,
                                .span = .{ .start = start - 2, .end = stream.iter.i },
                            };
                        },
                        '&', '{' => {
                            stream.state = .var_name;
                            return Token{
                                .kind = .raw_var_tag_start,
                                .span = .{ .start = start - 2, .end = stream.iter.i },
                            };
                        },
                        '!' => {
                            while (true) {
                                if (cur == 0) {
                                    // We still want to record eof token.
                                    stream.iter.i -= 1;
                                    return Token{
                                        .kind = .nonterminated_comment,
                                        .span = .{ .start = start, .end = stream.iter.i + 1 },
                                    };
                                }

                                const nxt = stream.nextCodepoint();
                                if (cur == '}' and nxt == '}') {
                                    break;
                                }
                                cur = nxt;
                            }
                            stream.state = .content;
                        },
                        else => |cp| {
                            stream.state = .var_name;
                            stream.iter.i -= std.unicode.utf8CodepointSequenceLength(cp) catch unreachable;
                            return Token{
                                .kind = .var_tag_start,
                                .span = .{ .start = start - 2, .end = start },
                            };
                        },
                    }
                },
                .var_name => switch (cur) {
                    '#', '&', '{', '^', '>', '~' => {
                        stream.state = .unexpected_symbol;
                        return Token{
                            .kind = .var_name,
                            .span = .{ .start = start, .end = stream.iter.i },
                        };
                    },
                    ' ', '\n', '\r', '\t' => {
                        var actualEnd = stream.iter.i - 1;
                        while (isWhitespace(cur)) : (cur = stream.nextCodepoint()) {}
                        stream.state = .closebrace1;
                        return Token{
                            .kind = .var_name,
                            .span = .{ .start = start - 1, .end = actualEnd },
                        };
                    },
                    '}' => {
                        stream.state = .closebrace1;
                        return Token{
                            .kind = .var_name,
                            .span = .{ .start = start - 1, .end = stream.iter.i - 1 },
                        };
                    },
                    else => {},
                },
                .closebrace1 => switch (cur) {
                    '}' => {
                        stream.state = .content;
                        return Token{
                            .kind = .tag_end,
                            .span = .{ .start = start - 1, .end = start + 1 },
                        };
                    },
                    else => {
                        stream.state = .content;
                        return Token{
                            .kind = .unexpected_symbol,
                            .span = .{ .start = start, .end = start + 1 },
                        };
                    },
                },
                .unexpected_symbol => {
                    return Token{
                        .kind = .unexpected_symbol,
                        .span = .{ .start = start, .end = start + 1 },
                    };
                },
            }
        }
    }
};

pub const Node = struct {
    main_token: usize,
    data: Data = undefined,

    pub const Data = union {
        tag: enum {
            var_include,
            raw_var_include,
            section_start,
            inv_section_start,
            partial_include,
            section_end,
        },
    };
};

pub const Parser = struct {
    /// The current token index.
    index: usize,
    buffer: []const u8,
    token_spans: []const Token.Span,
    token_kinds: []const Token.Kind,
    errors: std.ArrayList(Error),
    failed: bool = false,
    nodes: std.MultiArrayList(Node),
    allocator: std.mem.Allocator,
    /// A stack of tag **node** indices used to validate the opening and closing
    /// of tags.
    tag_stack: std.ArrayList(usize),

    pub fn deinit(p: *Parser) void {
        p.errors.deinit();
        p.nodes.deinit(p.allocator);
        p.tag_stack.deinit();
    }

    pub const E = error{ParseError};

    inline fn note(p: *Parser, e: Error) !void {
        try p.errors.append(e);
    }

    inline fn fail(p: *Parser, e: Error) !anyerror {
        if (!p.failed) p.failed = true;
        try p.errors.append(e);
        return error.ParseError;
    }

    pub const Error = struct {
        kind: Error.Kind,
        index: usize,
        extra_data: union {
            expected_data: struct { expected: Token.Kind, got: ?Token.Kind },
            unclosed_section_name: []const u8,
        } = undefined,

        pub const Kind = enum {
            expected_token,
            unmatched_section_tag_end,
            note_unclosed_section,

            pub fn isNote(kind: Kind) bool {
                return switch (kind) {
                    .note_unclosed_section => true,
                    else => false,
                };
            }
        };

        pub fn write(e: *const Error, p: *const Parser, writer: anytype) !void {
            switch (e.kind) {
                .expected_token => {
                    try writer.print("expected {s} but found {s}", .{ e.extra_data.expected_data.expected.symbol(), if (e.extra_data.expected_data.got) |kind| kind.symbol() else "nothing" });
                },

                .unmatched_section_tag_end => {
                    try writer.writeAll("unmatched section close");
                },
                .note_unclosed_section => {
                    const identTokIdx = p.nodes.items(.main_token)[e.index];
                    const identTokSpan = p.token_spans[identTokIdx];
                    try writer.print("a section named \"{s}\" had been opened here", .{p.buffer[identTokSpan.start..identTokSpan.end]});
                },
            }
        }
    };

    fn next(p: *Parser) ?usize {
        var cur = p.index;
        if (cur == p.token_kinds.len) return null;
        p.index += 1;
        return cur;
    }

    fn eat(p: *Parser, kind: Token.Kind) !usize {
        const nxt = p.next();
        return if (nxt != null and p.token_kinds[nxt.?] == kind) nxt.? else return try p.fail(Error{
            .kind = .expected_token,
            .index = nxt orelse p.token_kinds.len - 1,
            .extra_data = .{ .expected_data = .{ .expected = kind, .got = if (nxt) |idx| p.token_kinds[idx] else null } },
        });
    }

    inline fn addNode(p: *Parser, n: Node) !usize {
        try p.nodes.append(p.allocator, n);
        return p.nodes.len - 1;
    }

    inline fn isTagStart(tok: Token.Kind) bool {
        return switch (tok) {
            .var_tag_start, .raw_var_tag_start, .inv_section_tag_start, .section_tag_start, .section_tag_end_start, .partial_tag_start => true,
            else => false,
        };
    }

    /// Returns the index of the parsed node. Assumes the first node is a valid
    /// tag start kind.
    fn parseTag(p: *Parser) !usize {
        const tagStartKind = p.next() orelse unreachable;
        const identTok = try p.eat(.var_name);
        _ = try p.eat(.tag_end);
        const nodeIndex = try p.addNode(Node{
            .main_token = identTok,
            .data = .{
                .tag = switch (p.token_kinds[tagStartKind]) {
                    .var_tag_start => .var_include,
                    .raw_var_tag_start => .raw_var_include,
                    .section_tag_start => .section_start,
                    .section_tag_end_start => .section_end,
                    .inv_section_tag_start => .inv_section_start,
                    .partial_tag_start => .partial_include,
                    else => unreachable,
                },
            },
        });
        switch (p.token_kinds[tagStartKind]) {
            .section_tag_start => try p.tag_stack.append(nodeIndex),
            .section_tag_end_start => {
                var lastTagStartIdx = p.tag_stack.popOrNull() orelse return try p.fail(Error{
                    .kind = .unmatched_section_tag_end,
                    .index = tagStartKind,
                });

                const identTokA = identTok;
                const identTokB = p.nodes.items(.main_token)[lastTagStartIdx];

                const identTokASpan = p.token_spans[identTokA];
                const identTokBSpan = p.token_spans[identTokB];

                const identTokABytes = p.buffer[identTokASpan.start..identTokASpan.end];
                const identTokBBytes = p.buffer[identTokBSpan.start..identTokBSpan.end];

                if (!std.mem.eql(u8, identTokABytes, identTokBBytes)) {
                    try p.fail(Error{
                        .kind = .unmatched_section_tag_end,
                        .index = tagStartKind,
                    }) catch {};

                    try p.note(Error{
                        .kind = .note_unclosed_section,
                        .index = lastTagStartIdx,
                    });

                    return error.ParseError;
                }
            },
            else => {},
        }
        return nodeIndex;
    }

    pub fn parse(p: *Parser) !void {
        while (true) {
            const tok = p.next() orelse break;
            switch (p.token_kinds[tok]) {
                .eof => break,
                .content => _ = try p.addNode(Node{
                    .main_token = tok,
                }),
                .partial_tag_start, .section_tag_end_start, .raw_var_tag_start, .var_tag_start, .section_tag_start, .inv_section_tag_start => {
                    p.index -= 1;
                    _ = try p.parseTag();
                },
                else => unreachable,
            }
        }
    }
};

fn ccUnion(n: usize, writer: anytype) !void {
    while (n > 0) {
        if (n != 0) try writer.writeAll("||");
        try writer.print("cc{}", n);
    }
}

pub const CompilerConfig = struct {
    include_source_map: bool,
};

pub fn Compiler(comptime Config: CompilerConfig) type {
    _ = Config;
    return struct {
        pub fn compile(
            writer: anytype,
            p: *const Parser,
            _: anytype,
        ) !void {
            // var base64 = std.base64.Base64Decoder.init(std.base64.standard, null);
            // if (!Config.include_source_map) {
            //     _ = source_map_data;
            // } else if (!@hasField(@TypeOf(writer), "bytes_written")) {
            //     @compileError("The main writer must be a counting writer if you are using source maps.");
            // }
            try writer.writeAll("let c=(a,b,o=Object)=>o.assign(o.create(a),b);");
            try writer.writeAll("export function compile(context){let cc0=context,o=\"\",k;");
            var cc: u32 = 0;
            var nodeSlice = p.nodes.slice();
            for (nodeSlice.items(.main_token)) |mainTokIdx, i| {
                const main_token = p.token_kinds[mainTokIdx];

                if (main_token == .content) {
                    const sp = p.token_spans[mainTokIdx];
                    try writer.print("o=o+`{s}`;", .{p.buffer[sp.start..sp.end]});
                    continue;
                }

                var data = nodeSlice.items(.data)[i];
                switch (data.tag) {
                    .var_include => {
                        const sp = p.token_spans[mainTokIdx];
                        try writer.print("o=o+cc{}[`{s}`];", .{ cc, p.buffer[sp.start..sp.end] });
                    },
                    .raw_var_include => {
                        const sp = p.token_spans[mainTokIdx];
                        try writer.print("o=o+cc{}[`{s}`];", .{ cc, p.buffer[sp.start..sp.end] });
                    },
                    .section_start => {
                        const sp = p.token_spans[mainTokIdx];
                        try writer.print("for(let k=`{3s}`,cc{1}=c(cc{0},cc{0}[k]),cc{2},cci{1}=0,ccmi{1}=(0 in cc{1})?(cc{2}=cc{1}[0], cc{0}[k].length):+!!cc{1};cci{1}<ccmi{1};cci{1}++,cc{2}=cc{1}[cci{1}]){{", .{
                            cc,
                            cc + 1,
                            cc + 2,
                            p.buffer[sp.start..sp.end],
                        });
                        cc += 2;
                    },
                    .section_end => {
                        try writer.writeAll("}");
                        cc -= 2;
                    },
                    else => @panic("."),
                }
            }
            try writer.writeAll("return o;}");
        }
    };
}

fn expectTokens(input: []const u8, tokens: []const Token) !void {
    var stream = TokenStream{ .iter = .{ .bytes = input, .i = 0 } };
    var list = std.ArrayList(Token).init(std.testing.allocator);
    defer list.deinit();
    try list.ensureTotalCapacity(stream.iter.bytes.len / 16);
    while (true) {
        const tk = stream.next();
        try list.append(tk);
        if (tk.kind == .eof) break;
    }
    try std.testing.expectEqualSlices(Token, tokens, list.items);
}

test "mustache/tokenize-no-content" {
    try expectTokens("", &.{
        .{ .kind = .eof, .span = .{ .start = 0, .end = 0 } },
    });
}

test "mustache/tokenize-only-content" {
    try expectTokens("Hello, world!", &.{
        .{ .kind = .content, .span = .{ .start = 0, .end = 13 } },
        .{ .kind = .eof, .span = .{ .start = 13, .end = 13 } },
    });
}

test "mustache/tokenize-basic-var-name" {
    // no content
    try expectTokens("{{name}}", &.{
        .{ .kind = .var_tag_start, .span = .{ .start = 0, .end = 2 } },
        .{ .kind = .var_name, .span = .{ .start = 2, .end = 6 } },
        .{ .kind = .tag_end, .span = .{ .start = 6, .end = 8 } },
        .{ .kind = .eof, .span = .{ .start = 8, .end = 8 } },
    });

    // no leading content
    try expectTokens("{{name}} is awesome!", &.{
        .{ .kind = .var_tag_start, .span = .{ .start = 0, .end = 2 } },
        .{ .kind = .var_name, .span = .{ .start = 2, .end = 6 } },
        .{ .kind = .tag_end, .span = .{ .start = 6, .end = 8 } },
        .{ .kind = .content, .span = .{ .start = 8, .end = 20 } },
        .{ .kind = .eof, .span = .{ .start = 20, .end = 20 } },
    });

    // no trailing content
    try expectTokens("I love {{name}}", &.{
        .{ .kind = .content, .span = .{ .start = 0, .end = 7 } },
        .{ .kind = .var_tag_start, .span = .{ .start = 7, .end = 9 } },
        .{ .kind = .var_name, .span = .{ .start = 9, .end = 13 } },
        .{ .kind = .tag_end, .span = .{ .start = 13, .end = 15 } },
        .{ .kind = .eof, .span = .{ .start = 15, .end = 15 } },
    });

    // leading + trailing content
    try expectTokens("Hello, {{name}}.", &.{
        .{ .kind = .content, .span = .{ .start = 0, .end = 7 } },
        .{ .kind = .var_tag_start, .span = .{ .start = 7, .end = 9 } },
        .{ .kind = .var_name, .span = .{ .start = 9, .end = 13 } },
        .{ .kind = .tag_end, .span = .{ .start = 13, .end = 15 } },
        .{ .kind = .content, .span = .{ .start = 15, .end = 16 } },
        .{ .kind = .eof, .span = .{ .start = 16, .end = 16 } },
    });

    // inner whitespace
    try expectTokens("CONTENT{{& name }}", &.{
        .{ .kind = .content, .span = .{ .start = 0, .end = 7 } },
        .{ .kind = .raw_var_tag_start, .span = .{ .start = 7, .end = 10 } },
        .{ .kind = .var_name, .span = .{ .start = 11, .end = 15 } },
        .{ .kind = .tag_end, .span = .{ .start = 16, .end = 18 } },
        .{ .kind = .eof, .span = .{ .start = 18, .end = 18 } },
    });

    // inner whitespace
    try expectTokens("CONTENT{{/name}}", &.{
        .{ .kind = .content, .span = .{ .start = 0, .end = 7 } },
        .{ .kind = .section_tag_end_start, .span = .{ .start = 7, .end = 10 } },
        .{ .kind = .var_name, .span = .{ .start = 10, .end = 14 } },
        .{ .kind = .tag_end, .span = .{ .start = 14, .end = 16 } },
        .{ .kind = .eof, .span = .{ .start = 16, .end = 16 } },
    });
}
