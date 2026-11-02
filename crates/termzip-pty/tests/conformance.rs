//! What a program running in the PTY can count on: the terminal type and the size it was given (and a resize), UTF-8
//! both ways, Ctrl+C reaching the foreground job, and a big paste arriving whole.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use termzip_pty::{PtyOptions, PtySession};

/// Collects output until `marker` shows up or `timeout` passes. On Windows, ConPTY asks where the cursor is before
/// it will run anything, so that question is answered.
fn collect_until(
    session: &PtySession,
    output: &Receiver<Vec<u8>>,
    marker: &str,
    timeout: Duration,
) -> String {
    let deadline = Instant::now() + timeout;
    let mut received = Vec::new();
    while Instant::now() < deadline {
        if let Ok(bytes) = output.recv_timeout(Duration::from_millis(100)) {
            received.extend(bytes);
            if cfg!(windows) && received.windows(4).any(|window| window == b"\x1b[6n") {
                let _ = session.write(b"\x1b[1;1R");
            }
        }
        if String::from_utf8_lossy(&received).contains(marker) {
            break;
        }
    }
    String::from_utf8_lossy(&received).into_owned()
}

/// Collects everything until the program ends (the channel closes) or `timeout` passes.
#[cfg(unix)]
fn collect_all(output: &Receiver<Vec<u8>>, timeout: Duration) -> Vec<u8> {
    let deadline = Instant::now() + timeout;
    let mut received = Vec::new();
    while Instant::now() < deadline {
        match output.recv_timeout(Duration::from_millis(200)) {
            Ok(bytes) => received.extend(bytes),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    received
}

#[cfg(unix)]
fn sh(script: &str, rows: u16, columns: u16) -> (PtySession, Receiver<Vec<u8>>) {
    let options = PtyOptions {
        shell: Some("/bin/sh".into()),
        args: vec!["-c".to_owned(), script.to_owned()],
        rows,
        columns,
        ..PtyOptions::default()
    };
    PtySession::spawn(options, || {}).expect("a PTY with /bin/sh")
}

#[cfg(unix)]
#[test]
fn the_program_sees_the_terminal_type_and_the_size_it_was_given() {
    let (session, output) = sh(
        "stty size; echo \"$TERM|$COLORTERM|$TERM_PROGRAM\"; exit 0",
        31,
        101,
    );

    let text = collect_until(&session, &output, "TermZip", Duration::from_secs(10));

    assert!(text.contains("31 101"), "{text:?}");
    assert!(
        text.contains("xterm-256color|truecolor|TermZip"),
        "{text:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_resize_reaches_the_program() {
    let (session, output) = sh("sleep 1; stty size; exit 0", 24, 80);
    std::thread::sleep(Duration::from_millis(300));

    session.resize(40, 120, 0, 0).unwrap();
    let text = collect_until(&session, &output, "40 120", Duration::from_secs(10));

    assert!(text.contains("40 120"), "{text:?}");
}

#[cfg(unix)]
#[test]
fn text_in_utf8_goes_through_both_ways() {
    // The program prints what it read, so the text crossed the PTY in both directions.
    let (session, output) = sh(
        "stty raw -echo; head -c 18 | od -An -tx1; printf 'héllo ✓ 漢字\\n'; exit 0",
        24,
        80,
    );
    std::thread::sleep(Duration::from_millis(300));

    session.write("héllo ✓ 漢字\n".as_bytes()).unwrap();
    let text = collect_until(&session, &output, "漢字", Duration::from_secs(10));

    assert!(text.contains("héllo ✓ 漢字"), "{text:?}");
    // 68 c3 a9 6c 6c 6f ...: é arrived as two bytes, not as a latin-1 byte.
    let spaced = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(spaced.contains("68 c3 a9 6c 6c 6f"), "{text:?}");
}

#[cfg(unix)]
#[test]
fn control_c_interrupts_the_foreground_program() {
    let (session, output) = sh(
        "trap 'echo got-sigint; exit 0' INT; echo started; while :; do sleep 0.05; done",
        24,
        80,
    );
    collect_until(&session, &output, "started", Duration::from_secs(10));

    session.write(&[0x03]).unwrap();
    let text = collect_until(&session, &output, "got-sigint", Duration::from_secs(10));

    assert!(text.contains("got-sigint"), "{text:?}");
}

#[cfg(unix)]
#[test]
fn a_big_paste_arrives_whole() {
    // Raw mode, as a shell's line editor puts it: no line is cut at the tty's limit of a few KiB.
    let (session, output) = sh(
        "stty raw -echo; echo ready; head -c 300000 | wc -c; exit 0",
        24,
        80,
    );
    collect_until(&session, &output, "ready", Duration::from_secs(10));

    let paste = vec![b'x'; 300_000];
    for piece in paste.chunks(8192) {
        session.write(piece).unwrap();
    }
    let text = collect_until(&session, &output, "300000", Duration::from_secs(20));

    assert!(text.contains("300000"), "{text:?}");
}

#[cfg(unix)]
#[test]
fn the_end_of_the_program_closes_the_output() {
    let (_session, output) = sh("echo done; exit 7", 24, 80);

    let received = collect_all(&output, Duration::from_secs(10));

    assert!(String::from_utf8_lossy(&received).contains("done"));
    // The channel closed (collect_all returned on Disconnected, not on its timeout), so the reader has ended.
}

#[cfg(windows)]
#[test]
fn text_in_utf8_comes_out_of_conpty_as_utf8() {
    let options = PtyOptions {
        shell: Some("cmd.exe".into()),
        args: vec!["/C".to_owned(), "echo héllo ✓".to_owned()],
        ..PtyOptions::default()
    };
    let (session, output) = PtySession::spawn(options, || {}).expect("a ConPTY with cmd.exe");

    let text = collect_until(&session, &output, "llo", Duration::from_secs(15));

    // The code page of cmd decides what `é` and `✓` print as; what must hold is that the ASCII around them survives
    // and that what comes out is valid UTF-8 (`from_utf8_lossy` would have put U+FFFD in otherwise).
    assert!(text.contains("llo"), "{text:?}");
    assert!(!text.contains('\u{fffd}'), "{text:?}");
}

#[cfg(windows)]
#[test]
fn the_size_given_to_conpty_is_the_size_the_program_sees() {
    let options = PtyOptions {
        shell: Some("cmd.exe".into()),
        args: vec!["/C".to_owned(), "mode con".to_owned()],
        rows: 31,
        columns: 101,
        ..PtyOptions::default()
    };
    let (session, output) = PtySession::spawn(options, || {}).expect("a ConPTY with cmd.exe");

    let text = collect_until(&session, &output, "101", Duration::from_secs(15));

    assert!(text.contains("101"), "{text:?}");
}
