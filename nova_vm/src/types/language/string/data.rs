use wtf8::Wtf8Buf;

#[derive(Debug, Clone)]
pub struct StringHeapData {
    // pub utf16_len: Option<u32>,
    pub data: Wtf8Buf,
}

impl StringHeapData {
    pub fn from_str(str: &str) -> Self {
        StringHeapData {
            data: Wtf8Buf::from_str(str),
        }
    }

    // TODO: Implement literals
    // pub const fn from_literal(str: &'static str) -> Self {
    //     // Literals shorter than 7 bytes should b SmallString optimized.
    //     debug_assert!(str.len() > 7);
    //     StringHeapData {
    //         data: StringBuffer::Literal(str),
    //     }
    // }
}
