pub const INVALID_UTF8: &[u8] = &[b'{', b'"', 0x80, b'"', b':', b'0', b'}'];

pub const PARSER_SEEDS: &[(&str, &[u8])] = &[
    (
        "decoded-duplicate",
        include_bytes!("../parser-seeds/decoded-duplicate.seed"),
    ),
    (
        "deep-array",
        include_bytes!("../parser-seeds/deep-array.seed"),
    ),
    ("invalid-utf8", INVALID_UTF8),
    (
        "malformed-number",
        include_bytes!("../parser-seeds/malformed-number.seed"),
    ),
    (
        "unicode-escapes",
        include_bytes!("../parser-seeds/unicode-escapes.seed"),
    ),
    (
        "unique-members",
        include_bytes!("../parser-seeds/unique-members.seed"),
    ),
    (
        "unterminated-string",
        include_bytes!("../parser-seeds/unterminated-string.seed"),
    ),
    (
        "valid-object",
        include_bytes!("../parser-seeds/valid-object.seed"),
    ),
    (
        "wide-object",
        include_bytes!("../parser-seeds/wide-object.seed"),
    ),
];
