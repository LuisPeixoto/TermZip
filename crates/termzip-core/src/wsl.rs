//! Reading the answer of `wsl.exe -l -q`: the names of the installed WSL distributions.
//!
//! `wsl.exe` writes UTF-16 (little endian, sometimes with a byte order mark) even into a pipe, one name per line;
//! some setups ask for UTF-8 instead. When there is no distribution, or WSL is not set up, it prints a sentence
//! instead, which is no name and is dropped. The output is untrusted text: bounded, and never a panic.

use crate::profile::is_valid_remote_name;

/// The most output that is looked at.
pub const MAX_WSL_OUTPUT_BYTES: usize = 64 * 1024;
/// The most names returned.
pub const MAX_DISTRIBUTIONS: usize = 64;

/// Docker Desktop keeps two distributions of its own in the list; they are not places to open a terminal.
fn is_internal(name: &str) -> bool {
    name.to_ascii_lowercase().starts_with("docker-desktop")
}

/// The text of the output, whichever way `wsl.exe` encoded it.
fn decode(bytes: &[u8]) -> String {
    let bytes = &bytes[..bytes.len().min(MAX_WSL_OUTPUT_BYTES)];
    // UTF-16 text of plain letters has a zero byte beside every other byte; UTF-8 text has none.
    let utf16 = bytes.len() >= 2 && bytes.iter().skip(1).step_by(2).any(|byte| *byte == 0);
    if !utf16 {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    // A last odd byte, if any, is half a unit and is dropped.
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    String::from_utf16_lossy(&units)
}

/// The distribution names in the output of `wsl.exe -l -q`, in the order it lists them (the default one first).
pub fn parse_distributions(output: &[u8]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for line in decode(output).lines() {
        let name = line.trim().trim_start_matches('\u{feff}').trim();
        if is_valid_remote_name(name)
            && !is_internal(name)
            && !names.iter().any(|known| known == name)
        {
            names.push(name.to_owned());
        }
        if names.len() >= MAX_DISTRIBUTIONS {
            break;
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `wsl.exe` writes: UTF-16LE, CRLF between names.
    fn utf16(text: &str, bom: bool) -> Vec<u8> {
        let mut units: Vec<u16> = Vec::new();
        if bom {
            units.push(0xFEFF);
        }
        units.extend(text.encode_utf16());
        units.iter().flat_map(|unit| unit.to_le_bytes()).collect()
    }

    #[test]
    fn the_names_are_read_from_utf16_output_with_or_without_a_byte_order_mark() {
        for bom in [false, true] {
            let output = utf16("Ubuntu-22.04\r\nDebian\r\n", bom);
            assert_eq!(
                parse_distributions(&output),
                ["Ubuntu-22.04", "Debian"],
                "bom {bom}"
            );
        }
    }

    #[test]
    fn utf8_output_is_read_too() {
        assert_eq!(
            parse_distributions(b"Ubuntu\nDebian\n"),
            ["Ubuntu", "Debian"]
        );
    }

    #[test]
    fn docker_desktop_s_own_distributions_are_left_out() {
        let output = utf16("Ubuntu\r\ndocker-desktop\r\ndocker-desktop-data\r\n", false);
        assert_eq!(parse_distributions(&output), ["Ubuntu"]);
    }

    #[test]
    fn the_sentence_wsl_prints_when_there_is_nothing_is_not_a_name() {
        let output = utf16(
            "Windows Subsystem for Linux has no installed distributions.\r\n",
            false,
        );
        assert!(parse_distributions(&output).is_empty());
        assert!(parse_distributions(b"").is_empty());
    }

    #[test]
    fn a_name_is_listed_once_and_names_that_are_not_plain_are_dropped() {
        let output = utf16(
            "Ubuntu\r\nUbuntu\r\n-oops\r\nhas space\r\nok.name_1\r\n",
            false,
        );
        assert_eq!(parse_distributions(&output), ["Ubuntu", "ok.name_1"]);
    }

    #[test]
    fn hostile_output_never_panics_and_is_bounded() {
        for bytes in [
            vec![0u8],
            vec![0xff],
            vec![0xff, 0xfe, 0x00],
            vec![0x00; 1000],
            vec![0xd8, 0x00, 0xd8, 0x00],
            b"a\0b\0c".to_vec(),
            "é\u{202e}".repeat(1000).into_bytes(),
            utf16(&"x\r\n".repeat(100_000), false),
        ] {
            let names = parse_distributions(&bytes);
            assert!(names.len() <= MAX_DISTRIBUTIONS);
        }
    }

    #[test]
    fn there_is_a_limit_to_the_names() {
        let many: String = (0..500).map(|n| format!("d{n}\r\n")).collect();
        assert_eq!(
            parse_distributions(&utf16(&many, false)).len(),
            MAX_DISTRIBUTIONS
        );
    }
}
