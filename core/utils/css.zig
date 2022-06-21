const std = @import("std");

pub const CSSIdentEncoder = std.base64.Base64Encoder.init(std.base64.url_safe_alphabet_chars, null);
pub const CSSIdentDecoder = std.base64.Base64Decoder.init(std.base64.url_safe_alphabet_chars, null);

/// https://theartincode.stanis.me/008-djb2/
pub fn hash(s: []const u8) usize {
    var h: usize = 5381;
    for (s) |c| h = (h << 5) + h + c;
    return h;
}
