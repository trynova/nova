// Deferred for after MVP

pub const Token = struct {
    start: usize,
    kind: Kind,

    pub const Kind = enum {
        /// can be anywhere
        whitespace,
        /// @ident - includes the ident for minor perf boost
        at_rule_decl,

        /// != [Sass-specific]
        eq,
        /// == [Sass-specific]
        neq,

        lbrace,
        rbrace,

        lbrack,
        rbrack,

        lparen,
        rparen,

        amp,
        colon,
        comma,
        minus,

        ident,
        colon,

        unexpected_ident,

        /// Builtin function starts
        builtin_url,

        /// Media query starts
        at_media,
        at_keyframes,

        pub const IdentMap = std.ComptimeStringMap(Token.Kind, .{
            .{ "url", .builtin_url },
        });

        pub const AtRuleIdentMap = std.ComptimeStringMap(Token.Kind, .{
            .{ "at", .at_media },
            .{ "keyframes", .at_keyframes },
        });

        pub fn symbol(kind: Kind) []const u8 {
            return switch (kind) {
                .eof => "the end of input",
                else => @panic("todo"),
            };
        }
    };
};

pub const TokenStream = struct {
    iter: std.unicode.Utf8Iterator,
    state: State = .content,
    has_partial: bool = false,
    extra_data: union {} = undefined,

    const State = enum {
        container_member,
        ident_start,
        ident_continue,
        whitespace,
        after_container_member,
    };

    pub fn init(input: []const u8) TokenStream {
        return .{
            .iter = .{ .bytes = if (input.len > 2 and input[0] == 0xEF and input[1] == 0xBB and input[2] == 0xBF) input[2..] else input, .i = 0 },
            .state = .content,
            .has_partial = false,
        };
    }

    inline fn recall(stream: *TokenStream, n: anytype) void {
        stream.iter.i -= n;
    }

    inline fn totalRecall(stream: *TokenStream, cp: u21) void {
        stream.iter.i -= std.unicode.utf8CodepointSequenceLength(cp) catch unreachable;
    }

    inline fn nextCodepoint(stream: *TokenStream) u21 {
        return stream.iter.nextCodepoint() orelse 0;
    }

    /// https://drafts.csswg.org/css-syntax/#non-ascii-ident-code-point
    inline fn isNonASCIIIdentCodepoint(cp: u21) bool {
        return switch (cp) {
            '\u{00B7}', '\u{00C0}'...'\u{00D6}', '\u{00D8}'...'\u{00F6}', '\u{00F8}'...'\u{037D}', '\u{037F}'...'\u{1FFF}', '\u{200C}', '\u{200D}', '\u{203F}', '\u{2040}', '\u{2070}'...'\u{218F}', 0x2C00...0x2FEF, 0x3001...0xD7FF, 0xF900...0xFDCF, 0xFDF0...0xFFFD => true,
            else => |c| c >= 0x10000,
        };
    }

    /// https://drafts.csswg.org/css-syntax/#non-ascii-ident-code-point
    inline fn isIdentStartCodepoint(cp: u21) bool {
        return switch (cp) {
            'a'...'z', 'A'...'Z', '_' => true,
            else => isNonASCIIIdentCodepoint(cp),
        };
    }

    /// https://drafts.csswg.org/css-syntax/#ident-code-point
    inline fn isIdentCodepoint(cp: u21) bool {
        return switch (cp) {
            '0'...'9', '-' => true,
            else => isIdentStartCodepoint(cp),
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

        var kind: Token.Kind = undefined;

        loop: while (true) : (cur = stream.nextCodepoint()) {
            switch (stream.state) {
                .container_member => switch (cur) {
                    ' ', '\t', '\r', '\n' => stream.state = .whitespace,
                    '@' => {
                        stream.state = .ident_start;
                        kind = .at_rule_decl;
                    },
                },
                .whitespace => switch (cur) {
                    ' ', '\t', '\r', '\n' => {},
                    else => |cp| {
                        stream.totalRecall(cp);
                        break;
                    },
                },
                .ident_start => switch (cur) {
                    0 => @panic("todo"),
                    else => |cp| {
                        if (isIdentStartCodepoint(cp)) {
                            stream.state = .ident;
                            continue;
                        }
                        @panic("todo");
                    },
                },
                .ident => switch (cur) {
                    0 => @panic("todo"),
                    else => |cp| {
                        if (isIdentCodepoint(cp)) continue;
                        stream.totalRecall(cp);
                        if (kind == .at_rule_decl) {
                            const at_rule_kind = Token.Kind.AtRuleIdentMap.get(stream.iter.bytes[start + 1 .. stream.iter.i + 1]);
                            if (at_rule_kind) |k| {
                                kind = k;
                            }
                        }
                        break;
                    },
                },
            }
        }

        return Token{
            .start = start,
            .kind = kind,
        };
    }
};
