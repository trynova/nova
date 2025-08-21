use regex::bytes::RegexBuilder;
use wtf8::{CodePoint, Wtf8, Wtf8Buf};

fn main() {
    let mut text = Wtf8Buf::from_str("𠮷a𠮷b𠮷c👨‍👩‍👧‍👦d");
    text.push(CodePoint::from_u32(55362).unwrap());
    let text_cus = text.to_ill_formed_utf16().collect::<Vec<_>>();
    eprintln!("{text:?}\n{text_cus:?}\n{:?}", unsafe {
        core::mem::transmute::<&Wtf8, &[u8]>(&text)
    });
    eprintln!("{}-{}", 0x0128, 0xFFFF);
    let haystack = "\u{0128}\u{ffff}";
    let re = RegexBuilder::new(r".{5}[\u0128-\uffff]")
        .dot_matches_new_line(false)
        .case_insensitive(false)
        .unicode(true) // u / v
        .dot_matches_new_line(false)
        .octal(false) // TODO: !strict
        .build()
        .unwrap();
    eprintln!("{haystack}");
    for m in re.find_iter(haystack.as_bytes()) {
        eprintln!("{m:?}");
    }
}
