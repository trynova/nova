// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// A macro which matches a radix and on a match defines a constant called
/// RADIX to the appropriate radix.
macro_rules! with_radix {
    ($radix:ident, $expr:expr) => {
        match $radix {
            2 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(2);
                $expr
            }
            3 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(3);
                $expr
            }
            4 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(4);
                $expr
            }
            5 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(5);
                $expr
            }
            6 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(6);
                $expr
            }
            7 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(7);
                $expr
            }
            8 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(8);
                $expr
            }
            9 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(9);
                $expr
            }
            10 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(10);
                $expr
            }
            11 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(11);
                $expr
            }
            12 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(12);
                $expr
            }
            13 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(13);
                $expr
            }
            14 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(14);
                $expr
            }
            15 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(15);
                $expr
            }
            16 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(16);
                $expr
            }
            17 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(17);
                $expr
            }
            18 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(18);
                $expr
            }
            19 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(19);
                $expr
            }
            20 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(20);
                $expr
            }
            21 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(21);
                $expr
            }
            22 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(22);
                $expr
            }
            23 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(23);
                $expr
            }
            24 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(24);
                $expr
            }
            25 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(25);
                $expr
            }
            26 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(26);
                $expr
            }
            27 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(27);
                $expr
            }
            28 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(28);
                $expr
            }
            29 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(29);
                $expr
            }
            30 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(30);
                $expr
            }
            31 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(31);
                $expr
            }
            32 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(32);
                $expr
            }
            33 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(33);
                $expr
            }
            34 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(34);
                $expr
            }
            35 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(35);
                $expr
            }
            36 => {
                const RADIX: u128 = lexical::NumberFormatBuilder::from_radix(36);
                $expr
            }
            _ => unreachable!(),
        }
    };
}
pub(crate) use with_radix;

pub(crate) fn make_float_string_ascii_lowercase(str: &mut str) {
    match &*str {
        "NaN" | "Infinity" | "-Infinity" => (),
        _ => {
            str.make_ascii_lowercase();
        }
    }
}
