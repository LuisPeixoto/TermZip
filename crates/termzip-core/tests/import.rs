//! The importers against real theme files from upstream repositories, and against hostile input.

use std::fs;
use std::path::PathBuf;

use termzip_core::{Color, Theme, ThemeKind, built_in_theme, import_theme};

fn fixture_bytes(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/themes")
        .join(name);
    fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn import_fixture(name: &str) -> Theme {
    import_theme(name, &fixture_bytes(name)).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn hex(color: Color) -> String {
    color.to_hex()
}

/// Dracula and Catppuccin Mocha as every format spells them: the same palette must come out.
#[test]
fn dracula_reads_the_same_in_every_format() {
    let expected = built_in_theme("dracula").unwrap();
    for name in [
        "dracula-alacritty.toml",
        "dracula-kitty.conf",
        "dracula-windows-terminal.json",
    ] {
        let theme = import_fixture(name);
        assert_eq!(
            theme.colors.background, expected.colors.background,
            "{name}"
        );
        assert_eq!(
            theme.colors.foreground, expected.colors.foreground,
            "{name}"
        );
        assert_eq!(theme.colors.ansi, expected.colors.ansi, "{name}");
        assert_eq!(theme.colors.selection, expected.colors.selection, "{name}");
        assert_eq!(theme.appearance, ThemeKind::Dark, "{name}");
    }
}

#[test]
fn the_iterm2_scheme_is_read_from_float_components() {
    // Upstream's iTerm2 file uses a darker background and plain greys; these values were checked against
    // Python's plistlib, not against the other Dracula files.
    let theme = import_fixture("dracula.itermcolors");

    assert_eq!(hex(theme.colors.background), "#1e1f29");
    assert_eq!(hex(theme.colors.foreground), "#e6e6e6");
    assert_eq!(hex(theme.colors.cursor), "#bbbbbb");
    assert_eq!(hex(theme.colors.selection), "#44475a");
    // 0.3333333432674408 and 1.0 are #55 and #ff.
    assert_eq!(hex(theme.colors.ansi[1]), "#ff5555");
    assert_eq!(hex(theme.colors.ansi[0]), "#000000");
    assert_eq!(theme.appearance, ThemeKind::Dark);
    assert_eq!(theme.name, "dracula");
}

#[test]
fn catppuccin_mocha_reads_the_same_in_every_format() {
    let expected = built_in_theme("catppuccin-mocha").unwrap();
    for name in [
        "catppuccin-mocha-kitty.conf",
        "catppuccin-mocha-alacritty.toml",
        "catppuccin-mocha-windows-terminal.json",
    ] {
        let theme = import_fixture(name);
        assert_eq!(
            theme.colors.background, expected.colors.background,
            "{name}"
        );
        assert_eq!(
            theme.colors.foreground, expected.colors.foreground,
            "{name}"
        );
        assert_eq!(theme.colors.cursor, expected.colors.cursor, "{name}");
        assert_eq!(theme.colors.ansi, expected.colors.ansi, "{name}");
    }
}

#[test]
fn ghostty_uses_bare_hex_and_palette_lines() {
    let theme = import_fixture("catppuccin-mocha-ghostty");

    assert_eq!(hex(theme.colors.background), "#1e1e2e");
    assert_eq!(hex(theme.colors.cursor), "#f5e0dc");
    assert_eq!(
        theme.colors.cursor_text.map(hex).as_deref(),
        Some("#11111b")
    );
    assert_eq!(hex(theme.colors.selection), "#353749");
    // This file lists white and bright white the other way round from the others, and it is kept as is.
    assert_eq!(hex(theme.colors.ansi[7]), "#a6adc8");
    assert_eq!(hex(theme.colors.ansi[15]), "#bac2de");
    assert_eq!(theme.name, "catppuccin mocha ghostty");
}

#[test]
fn a_scheme_names_itself_when_the_format_has_a_name() {
    assert_eq!(
        import_fixture("dracula-windows-terminal.json").name,
        "Dracula"
    );
    // Kitty and Alacritty files have none, so the file name is used.
    assert_eq!(import_fixture("dracula-kitty.conf").name, "dracula kitty");
}

#[test]
fn words_where_a_cursor_color_could_be_are_ignored() {
    // Kitty writes `cursor_text_color background`, Alacritty `text = "CellForeground"`.
    let kitty = import_fixture("dracula-kitty.conf");
    assert_eq!(kitty.colors.cursor_text, None);
    let alacritty = import_fixture("dracula-alacritty.toml");
    assert_eq!(hex(alacritty.colors.cursor), "#f8f8f2");
}

#[test]
fn what_a_file_leaves_out_comes_from_the_termzip_palette() {
    let dark = import_theme("bare.conf", b"background #101010\nforeground #eeeeee\n").unwrap();
    let termzip = built_in_theme("termzip-dark").unwrap();
    assert_eq!(dark.colors.ansi, termzip.colors.ansi);
    assert_eq!(dark.colors.cursor, dark.colors.foreground);
    assert!(
        dark.colors.selection.a < 255,
        "a faint wash, not a solid block"
    );

    let light = import_theme("bare.conf", b"background #f5f5f5\nforeground #222222\n").unwrap();
    assert_eq!(light.appearance, ThemeKind::Light);
    assert_eq!(
        light.colors.ansi,
        built_in_theme("termzip-light").unwrap().colors.ansi
    );
}

#[test]
fn a_termzip_theme_is_imported_as_it_is() {
    let original = built_in_theme("nord").unwrap();
    let text = original.to_toml().unwrap();

    assert_eq!(
        import_theme("nord.toml", text.as_bytes()).unwrap(),
        original
    );
}

#[test]
fn a_whole_settings_file_gives_the_scheme_named_like_the_file_or_the_first() {
    let settings = r##"{"schemes":[
        {"name":"One","background":"#000000","foreground":"#ffffff"},
        {"name":"Two","background":"#ffffff","foreground":"#000000"}]}"##;

    assert_eq!(
        import_theme("settings.json", settings.as_bytes())
            .unwrap()
            .name,
        "One"
    );
    let second = import_theme("two.json", settings.as_bytes()).unwrap();
    assert_eq!(second.name, "Two");
    assert_eq!(second.appearance, ThemeKind::Light);
    let list = format!("[{}]", &settings[12..settings.len() - 2]);
    assert!(import_theme("list.json", list.as_bytes()).is_ok());
}

#[test]
fn the_old_alacritty_hex_spelling_is_accepted() {
    let text = "[colors.primary]\nbackground = '0x1d1f21'\nforeground = '0xc5c8c6'\n[colors.normal]\nred = '0xcc6666'\n";
    let theme = import_theme("old.toml", text.as_bytes()).unwrap();

    assert_eq!(hex(theme.colors.background), "#1d1f21");
    assert_eq!(hex(theme.colors.ansi[1]), "#cc6666");
}

#[test]
fn a_byte_order_mark_and_windows_line_endings_do_not_matter() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"background #112233\r\nforeground #ddeeff\r\ncolor1 #ff0000\r\n");
    let theme = import_theme("crlf.conf", &bytes).unwrap();

    assert_eq!(hex(theme.colors.background), "#112233");
    assert_eq!(hex(theme.colors.ansi[1]), "#ff0000");
}

#[test]
fn ghostty_entries_past_the_16th_color_are_ignored() {
    let mut text = String::from("background = 000000\nforeground = ffffff\n");
    for index in 0..300 {
        text.push_str(&format!("palette = {index}=#0000ff\n"));
    }
    let theme = import_theme("many.conf", text.as_bytes()).unwrap();

    assert_eq!(hex(theme.colors.ansi[15]), "#0000ff");
}

#[test]
fn an_iterm2_component_out_of_range_is_clamped() {
    let plist = |value: &str| {
        format!(
            r#"<?xml version="1.0"?><plist version="1.0"><dict>
            <key>Background Color</key><dict><key>Red Component</key><real>{value}</real>
            <key>Green Component</key><real>0</real><key>Blue Component</key><real>0</real></dict>
            <key>Foreground Color</key><dict><key>Red Component</key><real>1</real>
            <key>Green Component</key><real>1</real><key>Blue Component</key><real>1</real></dict>
            </dict></plist>"#
        )
    };

    let loud = import_theme("x.itermcolors", plist("7.5").as_bytes()).unwrap();
    assert_eq!(hex(loud.colors.background), "#ff0000");
    let negative = import_theme("x.itermcolors", plist("-3").as_bytes()).unwrap();
    assert_eq!(hex(negative.colors.background), "#000000");
}

fn assert_sentence(result: Result<Theme, String>, label: &str) {
    let error = result.expect_err(label);
    assert!(
        error.ends_with('.') && error.len() > 12,
        "{label}: not a sentence: {error:?}"
    );
}

#[test]
fn files_that_are_not_themes_are_explained_not_crashed_on() {
    assert_sentence(import_theme("a.conf", b""), "empty");
    assert_sentence(import_theme("a.conf", b"   \n\n  "), "blank");
    assert_sentence(
        import_theme("a.conf", b"hello world\nnothing here\n"),
        "prose",
    );
    assert_sentence(
        import_theme("a.png", &[0x89, b'P', b'N', b'G', 0xff, 0xfe, 0x00]),
        "binary",
    );
    assert_sentence(
        import_theme("a.toml", b"[colors.primary]\n"),
        "empty alacritty",
    );
    assert_sentence(import_theme("a.json", b"{}"), "empty json");
    assert_sentence(import_theme("a.json", b"{\"schemes\": []}"), "no schemes");
    assert_sentence(import_theme("a.json", b"[1, 2, 3]"), "not objects");
    assert_sentence(import_theme("a.json", b"\"just a string\""), "string");
}

#[test]
fn truncated_files_are_refused() {
    for name in [
        "dracula.itermcolors",
        "dracula-windows-terminal.json",
        "dracula-alacritty.toml",
    ] {
        let bytes = fixture_bytes(name);
        for keep in [bytes.len() / 3, bytes.len() / 2, bytes.len() - 20] {
            // A cut text file can still be a valid, smaller theme; it must never panic.
            let _ = import_theme(name, &bytes[..keep]);
        }
    }
    let json = fixture_bytes("dracula-windows-terminal.json");
    assert_sentence(import_theme("a.json", &json[..json.len() / 2]), "cut json");
    let plist = fixture_bytes("dracula.itermcolors");
    assert_sentence(
        import_theme("a.itermcolors", &plist[..plist.len() / 2]),
        "cut plist",
    );
}

#[test]
fn wrong_types_and_bad_colors_name_what_is_wrong() {
    let number =
        import_theme("a.json", br##"{"background": 5, "foreground": "#fff"}"##).unwrap_err();
    assert!(number.contains("background"), "{number}");

    let word = import_theme(
        "a.json",
        br##"{"background": "red", "foreground": "#fff"}"##,
    )
    .unwrap_err();
    assert!(
        word.contains("“red”") && word.contains("background"),
        "{word}"
    );

    let line = import_theme(
        "a.conf",
        b"background #000000\nforeground #ffffff\ncolor3 nonsense\n",
    )
    .unwrap_err();
    assert!(
        line.starts_with("Line 3:") && line.contains("nonsense"),
        "{line}"
    );

    let palette = import_theme("a.conf", b"palette = three=#000000\n").unwrap_err();
    assert!(palette.contains("three"), "{palette}");

    let missing = import_theme("a.conf", b"color1 #ff0000\n").unwrap_err();
    assert!(missing.contains("background"), "{missing}");
}

#[test]
fn an_iterm2_color_that_is_not_a_number_is_refused() {
    let text = r#"<?xml version="1.0"?><plist version="1.0"><dict>
        <key>Background Color</key><dict><key>Red Component</key><string>lots</string>
        <key>Green Component</key><real>0</real><key>Blue Component</key><real>0</real></dict>
        </dict></plist>"#;
    assert_sentence(
        import_theme("a.itermcolors", text.as_bytes()),
        "string component",
    );

    let not_a_color = r#"<?xml version="1.0"?><plist version="1.0"><dict>
        <key>Background Color</key><string>red</string></dict></plist>"#;
    assert_sentence(
        import_theme("a.itermcolors", not_a_color.as_bytes()),
        "string color",
    );

    let nan = r#"<?xml version="1.0"?><plist version="1.0"><dict>
        <key>Background Color</key><dict><key>Red Component</key><real>nan</real>
        <key>Green Component</key><real>0</real><key>Blue Component</key><real>0</real></dict>
        <key>Foreground Color</key><dict><key>Red Component</key><real>1</real>
        <key>Green Component</key><real>1</real><key>Blue Component</key><real>1</real></dict>
        </dict></plist>"#;
    assert_sentence(import_theme("a.itermcolors", nan.as_bytes()), "nan");
}

#[test]
fn a_file_over_the_limit_is_refused_before_it_is_read() {
    let huge = vec![b'#'; termzip_core::MAX_THEME_BYTES + 1];
    let error = import_theme("huge.conf", &huge).unwrap_err();
    assert!(error.contains("larger than"), "{error}");
}

#[test]
fn deeply_nested_input_cannot_overflow_the_stack() {
    let json = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    assert_sentence(import_theme("deep.json", json.as_bytes()), "json");

    let objects = format!("{}1{}", "{\"a\":".repeat(5_000), "}".repeat(5_000));
    assert_sentence(import_theme("deep.json", objects.as_bytes()), "objects");

    let plist = format!(
        r#"<?xml version="1.0"?><plist version="1.0">{}{}</plist>"#,
        "<dict><key>k</key>".repeat(5_000),
        "</dict>".repeat(5_000)
    );
    assert_sentence(import_theme("deep.itermcolors", plist.as_bytes()), "plist");

    let toml = format!("a = {}{}", "[".repeat(5_000), "]".repeat(5_000));
    assert_sentence(import_theme("deep.toml", toml.as_bytes()), "toml");
}
