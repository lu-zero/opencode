//! Platform launcher for URLs and paths.

use base64::Engine;
use std::process::Command;

/// `openUrl`'s check: the normalized `href` of an http or https link, or the error to show. Check a URL with this
/// before [`command`], which also starts programs and opens files.
pub fn web_url(input: &str) -> Result<String, String> {
    match opencode_api_types::js::parse_url(input) {
        Err(error @ opencode_api_types::js::UrlError::NonAsciiHost(_)) => Err(error.to_string()),
        url => url
            .ok()
            .filter(|url| matches!(url.scheme(), "http" | "https"))
            .map(String::from)
            .ok_or_else(|| format!("Only http and https links can be opened in the browser: {input}")),
    }
}

/// Commands that can open `target`, in order: `$BROWSER` candidates first,
/// then the platform opener. The caller sets stdio and spawns them.
pub fn commands(target: &str) -> Vec<Command> {
    let browser = if cfg!(windows) {
        None
    } else {
        std::env::var("BROWSER").ok().filter(|value| !value.trim().is_empty())
    };
    let mut out: Vec<Command> = browser
        .as_deref()
        .into_iter()
        .flat_map(|value| browser_entries(target, value))
        .map(|(program, args)| configured(program, args))
        .collect();
    if out.is_empty() {
        out.push(system_command(target));
    }
    out
}

/// The first opener, for callers that cannot try a fallback.
pub fn command(target: &str) -> Command {
    commands(target).into_iter().next().expect("an opener")
}

fn configured(program: String, args: Vec<String>) -> Command {
    let mut command = Command::new(program);
    // `open` detaches `xdg-open`, so a browser it starts does not stop with this client's group on Ctrl+Z.
    #[cfg(all(unix, not(target_os = "macos")))]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    command.args(args);
    command
}

fn system_command(target: &str) -> Command {
    let system_root = std::env::var("SYSTEMROOT").or_else(|_| std::env::var("windir")).ok();
    let (program, args) = opener(std::env::consts::OS, target, system_root.as_deref());
    configured(program, args)
}

/// `$BROWSER` follows the Rust `webbrowser` contract: colon-separated
/// candidates, whitespace-separated arguments, `%s`/`%c`/`%%` field codes,
/// and the URL appended when no `%s` names a place for it.
fn browser_entries(target: &str, browser: &str) -> Vec<(String, Vec<String>)> {
    browser
        .split(':')
        .filter_map(|entry| {
            let mut parts = entry.split_whitespace();
            let program = parts.next()?;
            let mut args = Vec::new();
            let mut placed = false;
            for part in parts {
                let (expanded, has_url) = expand_field(part, target);
                placed |= has_url;
                args.push(expanded);
            }
            if !placed {
                args.push(target.to_owned());
            }
            Some((program.to_owned(), args))
        })
        .collect()
}

fn expand_field(part: &str, target: &str) -> (String, bool) {
    let mut out = String::new();
    let mut placed = false;
    let mut chars = part.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('s') => {
                out.push_str(target);
                placed = true;
            }
            Some('c') => out.push(':'),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    (out, placed)
}

/// The program and arguments for `os`. On Windows, `open` runs PowerShell's `Start` on the target, single-quoted and
/// Base64-encoded, so no shell parses `&`, `|`, `^` or `%VAR%` in it.
fn opener(os: &str, target: &str, system_root: Option<&str>) -> (String, Vec<String>) {
    match os {
        "macos" => ("open".to_owned(), vec![target.to_owned()]),
        "windows" => {
            // `powerShellPath` and `executePowerShell` of `powershell-utils`.
            let root = system_root.unwrap_or(r"C:\Windows");
            // `escapeArgument`: PowerShell ends a single-quoted string at any of these quotes, so double each.
            let quoted: String = target
                .chars()
                .flat_map(|ch| match ch {
                    '\'' | '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => vec![ch, ch],
                    _ => vec![ch],
                })
                .collect();
            let script = format!("$ProgressPreference = 'SilentlyContinue'; Start '{quoted}'");
            let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let encoded = base64::engine::general_purpose::STANDARD.encode(utf16);
            let args = ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand"];
            let mut args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
            args.push(encoded);
            (format!(r"{root}\System32\WindowsPowerShell\v1.0\powershell.exe"), args)
        }
        _ => ("xdg-open".to_owned(), vec![target.to_owned()]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(encoded: &str) -> String {
        let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
        let units: Vec<u16> = bytes.chunks(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
        String::from_utf16(&units).unwrap()
    }

    #[test]
    fn only_web_links_pass_as_urls() {
        assert_eq!(web_url("HTTPS://opencode.ai"), Ok("https://opencode.ai/".to_owned()));
        for input in [r"C:\Windows\System32\calc.exe", "calc", "file:///etc/passwd", "ms-settings:"] {
            assert_eq!(web_url(input), Err(format!("Only http and https links can be opened in the browser: {input}")));
        }
    }

    #[test]
    fn unix_passes_the_target_as_one_argument() {
        assert_eq!(opener("linux", "a b&c", None), ("xdg-open".to_owned(), vec!["a b&c".to_owned()]));
        assert_eq!(opener("macos", "https://x.y", None), ("open".to_owned(), vec!["https://x.y".to_owned()]));
    }

    #[test]
    fn windows_starts_the_target_through_encoded_powershell() {
        let (program, args) = opener("windows", "https://opencode.ai", Some(r"D:\Win"));
        assert_eq!(program, r"D:\Win\System32\WindowsPowerShell\v1.0\powershell.exe");
        assert_eq!(args[..5], ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand"]);
        // The `open` package's encoding of the same command.
        assert_eq!(
            args[5],
            "JABQAHIAbwBnAHIAZQBzAHMAUAByAGUAZgBlAHIAZQBuAGMAZQAgAD0AIAAnAFMAaQBsAGUAbgB0AGwAeQBDAG8AbgB0AGkAbgB1AGUAJwA7ACAAUwB0AGEAcgB0ACAAJwBoAHQAdABwAHMAOgAvAC8AbwBwAGUAbgBjAG8AZABlAC4AYQBpACcA"
        );
        assert_eq!(opener("windows", "x", None).0, r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
    }

    #[test]
    fn windows_quotes_the_target_for_powershell() {
        let cases = [
            (r"C:\Users\A B\file.md", r"'C:\Users\A B\file.md'"),
            ("https://x.y/?a=1&b=2", "'https://x.y/?a=1&b=2'"),
            (r"C:\a^b\%TEMP%\c", r"'C:\a^b\%TEMP%\c'"),
            (r#"it's "here""#, r#"'it''s "here"'"#),
            ("$(calc)", "'$(calc)'"),
            // `escapeArgument` of `powershell-utils`: PowerShell also ends a string at a typographic single quote.
            ("C:\\Ada\u{2019}s notes", "'C:\\Ada\u{2019}\u{2019}s notes'"),
            ("C:\\Users\\Bob\u{2019}s; calc #", "'C:\\Users\\Bob\u{2019}\u{2019}s; calc #'"),
            ("\u{2018}a\u{201A}b\u{201B}", "'\u{2018}\u{2018}a\u{201A}\u{201A}b\u{201B}\u{201B}'"),
        ];
        for (target, quoted) in cases {
            let args = opener("windows", target, None).1;
            assert_eq!(args.len(), 6);
            assert_eq!(decode(&args[5]), format!("$ProgressPreference = 'SilentlyContinue'; Start {quoted}"));
        }
    }

    #[test]
    fn browser_entries_append_the_url_and_expand_field_codes() {
        let entries = browser_entries("https://example.com", "browser a%cb %% %s");
        assert_eq!(
            entries,
            vec![("browser".to_owned(), vec!["a:b".to_owned(), "%".to_owned(), "https://example.com".to_owned()])]
        );
    }

    #[test]
    fn browser_entries_keep_candidates_in_order() {
        let entries = browser_entries("https://example.com", "first:second --tab");
        assert_eq!(
            entries,
            vec![
                ("first".to_owned(), vec!["https://example.com".to_owned()]),
                ("second".to_owned(), vec!["--tab".to_owned(), "https://example.com".to_owned()]),
            ]
        );
    }
}
