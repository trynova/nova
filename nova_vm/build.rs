// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

fn replace_invalid_key_characters(string: &str) -> String {
    let mut string = string.to_owned();

    if string == " " {
        return "__".to_string();
    }

    // If the first character is a number or a hyphen, prefix the string with an underscore.
    if let Some(first_char) = string.chars().next()
        && (first_char.is_numeric() || first_char == '-')
    {
        string = format!("_{string}");
    }

    string.replace(['[', ']', '(', ')', ' ', '.', '-', '*'], "_")
}

fn gen_builtin_strings() -> std::io::Result<Vec<u8>> {
    use small_string::SmallString;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open("src/builtin_strings")?;
    let reader = BufReader::new(file);
    // Use line count from builtin_strings
    let mut strings = Vec::with_capacity(256);

    for line in reader.lines() {
        let line = line.unwrap();
        if strings.contains(&line) {
            panic!("Duplicate strings {line}");
        }
        strings.push(line);
    }

    // Sizes measured from the final output plus a bit to allow for growth.
    let mut module = String::with_capacity(120_000);
    let mut list = String::with_capacity(10_000);
    let mut struct_def = String::with_capacity(50_000);
    let mut struct_const = String::with_capacity(54_000);

    module.push_str(
        "\
const fn get_index(n: &'static str) -> usize {
    let mut i = 0;
    'main: loop {
        let candidate = BUILTIN_STRINGS_LIST[i];
        if n.len() != candidate.len() {
            i += 1;
            continue 'main;
        }
        let n = n.as_bytes();
        let candidate = candidate.as_bytes();
        let mut j = 0;
        loop {
            if j == n.len() {
                break;
            }
            if n[j] != candidate[j] {
                i += 1;
                continue 'main;
            }
            j += 1;
        }
        return i;
    }
}\n\n\
    ",
    );

    // List
    list.push_str("pub const BUILTIN_STRINGS_LIST: &[&str] = &[\n");

    // Struct definition
    struct_def.push_str("#[allow(non_snake_case)]\npub struct BuiltinStrings {\n");

    // Struct instantiation
    struct_const.push_str("pub const BUILTIN_STRING_MEMORY: BuiltinStrings = BuiltinStrings {\n");
    for string in &strings {
        let (cfg, string) = if string.starts_with("#[cfg(") {
            let end_index = string.find(']').expect(
                "Builtin string started with feature attribute brackets but did not close it",
            );
            let (cfg, string) = string.split_at(end_index + 1);
            (format!("    {cfg}\n"), string)
        } else {
            ("".to_string(), string.as_ref())
        };
        if string.contains("#[") {
            panic!(
                "Builtin string still contains conditional compilation after split: \"{string}\""
            );
        }

        // List
        let is_small_string = SmallString::try_from(string).is_ok();
        if !is_small_string {
            // Do not output small strings into the builtin strings array.
            if !cfg.is_empty() {
                list.push_str(&cfg);
            }
            list.push_str(&format!("    \"{string}\",\n"));
        }

        // Struct definition
        struct_def.push_str("    /// ```js\n");
        struct_def.push_str(&format!("    /// \"{string}\"\n"));
        struct_def.push_str("    /// ```\n");
        if !cfg.is_empty() {
            struct_def.push_str(&cfg);
        }
        let escaped_string = replace_invalid_key_characters(string);
        struct_def.push_str("    pub r#");
        struct_def.push_str(&escaped_string);
        struct_def.push_str(": String<'static>,\n");

        // Struct instantiation
        if !cfg.is_empty() {
            struct_const.push_str(&cfg);
        }
        struct_const.push_str("    r#");
        struct_const.push_str(&escaped_string);
        if is_small_string {
            struct_const
                .push_str(": String::SmallString(unsafe { SmallString::from_str_unchecked(\"");
            struct_const.push_str(string);
            struct_const.push_str("\") }),\n");
        } else {
            struct_const
                .push_str(": String::String(HeapString(BaseIndex::from_index_const(get_index(");
            struct_const.push_str(&format!("\"{string}\")))),\n"));
        }
    }
    list.push_str("];\n\n");
    struct_def.push_str("}\n\n");
    struct_const.push_str("};\n");

    module.extend([list, struct_def, struct_const]);
    Ok(module.into_bytes())
}

fn main() {
    use std::env;
    use std::fs;
    use std::path::Path;

    println!("cargo:rerun-if-changed=src/builtin_strings");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("builtin_strings.rs");
    let builtin_strings_data = gen_builtin_strings().unwrap();
    fs::write(dest_path, builtin_strings_data).unwrap();
}
