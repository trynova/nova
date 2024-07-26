// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use small_string::SmallString;
use std::fs::File;
use std::io::{self, prelude::*, BufReader};

fn replace_invalid_key_characters(string: &str) -> String {
    let mut string = string.to_owned();

    if string == " " {
        return "__".to_string();
    }

    // If the first character is a number or a hyphen, prefix the string with an underscore.
    if let Some(first_char) = string.chars().next() {
        if first_char.is_numeric() || first_char == '-' {
            string = format!("_{}", string);
        }
    }

    string.replace(['[', ']', '(', ')', ' ', '.', '-', '*'], "_")
}

fn gen_builtin_strings() -> io::Result<Vec<u8>> {
    let file = File::open("src/builtin_strings")?;
    let reader = BufReader::new(file);
    // Use line count from builtin_strings
    let mut strings = Vec::with_capacity(256);

    let mut i: u32 = 0;
    for line in reader.lines() {
        let line = line.unwrap();
        if strings.contains(&line) {
            panic!("Duplicate strings {}", line);
        }
        if SmallString::try_from(line.as_str()).is_err() {
            i += 1;
        }
        strings.push(line);
    }

    let array_size = format!("{}", i);
    let mut output = String::with_capacity(2048);
    output.push_str("pub const BUILTIN_STRINGS_LIST: [&str; ");
    output.push_str(&array_size);
    output.push_str("] = [\n");
    for string in &strings {
        if SmallString::try_from(string.as_str()).is_ok() {
            // Do not output small strings into the builtin strings array.
            continue;
        }
        output.push_str("    \"");
        output.push_str(string);
        output.push_str("\",\n");
    }
    output.push_str("];\n\n#[allow(non_snake_case)]\npub struct BuiltinStrings {\n");
    for string in &strings {
        output.push_str("    /// ```js\n");
        output.push_str(&format!("    /// \"{}\"\n", string));
        output.push_str("    /// ```\n");
        output.push_str("    pub r#");
        output.push_str(&replace_invalid_key_characters(string));
        output.push_str(": String,\n");
    }
    output.push_str("}\n\npub const BUILTIN_STRING_MEMORY: BuiltinStrings = BuiltinStrings {\n");
    let mut i: u32 = 0;
    for string in strings.iter() {
        output.push_str("    r#");
        output.push_str(&replace_invalid_key_characters(string));
        if SmallString::try_from(string.as_str()).is_ok() {
            output.push_str(": crate::ecmascript::types::String::SmallString(SmallString::from_str_unchecked(\"");
            output.push_str(string.as_str());
            output.push_str("\")),\n");
        } else {
            output.push_str(
                ": crate::ecmascript::types::String::String(HeapString(StringIndex::from_index(",
            );
            output.push_str(&format!("{}", i));
            output.push_str("))),\n");
            i += 1;
        }
    }
    output.push_str("};\n");

    Ok(output.into_bytes())
}
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/builtin_strings");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("builtin_strings.rs");
    let builtin_strings_data = gen_builtin_strings().unwrap();
    fs::write(dest_path, builtin_strings_data).unwrap();
}
