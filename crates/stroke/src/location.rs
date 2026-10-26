//! URL 入力欄に打った文字を、開く URL に直す。
//!
//! 開いてよい URL の規則（`stroke_engine::policy`）に合わせる。`http:` は手元のサーバー以外では
//! `https:` に直し、`file:` などの開けないものは、検索に回さずに断る（打った文字が検索エンジンに
//! 漏れないように）。

use std::net::Ipv4Addr;

use stroke_engine::policy::{self, Blocked};
use url::Url;

/// URL でない入力を渡す検索エンジン。`{}` に検索語が入る。設定で変えられるようにするのは Phase 6
const SEARCH_URL: &str = "https://duckduckgo.com/html/?q={}";

/// URL として読むスキーム。これ以外の「名前:」で始まる入力（`localhost:8000` など）は、
/// スキームではなくホスト名と番号とみなす
const KNOWN_SCHEMES: &[&str] = &[
    "http",
    "https",
    "about",
    "file",
    "data",
    "javascript",
    "blob",
    "ftp",
    "ws",
    "wss",
    "mailto",
    "view-source",
];

#[derive(Debug, PartialEq, Eq)]
pub enum Rejected {
    /// 何も打っていない
    Empty,
    /// 開いてはいけない URL
    Blocked(Blocked),
}

/// 入力を開く URL に直す。URL に見えなければ検索の URL にする。
pub fn resolve(input: &str) -> Result<Url, Rejected> {
    let input = input.trim();
    if input.is_empty() {
        return Err(Rejected::Empty);
    }
    // 手元のファイルのパス
    if input.starts_with('/') {
        return Err(Rejected::Blocked(Blocked::Scheme));
    }
    let url = match as_url(input) {
        Some(url) => upgrade_to_https(url),
        None => {
            let query: String = url::form_urlencoded::byte_serialize(input.as_bytes()).collect();
            Url::parse(&SEARCH_URL.replace("{}", &query)).expect("SEARCH_URL は正しい URL")
        }
    };
    policy::check(&url, true).map_err(Rejected::Blocked)?;
    Ok(url)
}

fn as_url(input: &str) -> Option<Url> {
    // 空白を含むものは URL として打ったものではない
    if input.contains(char::is_whitespace) {
        return None;
    }
    if let Ok(url) = Url::parse(input)
        && KNOWN_SCHEMES.contains(&url.scheme())
    {
        return Some(url);
    }
    // スキームを省いたもの。ホスト名らしく見えるときだけ URL とみなす
    let authority = input.split(['/', '?', '#']).next().unwrap_or_default();
    let host = match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => host,
        _ => authority,
    };
    let local = host == "localhost" || host.parse::<Ipv4Addr>().is_ok();
    if !local && !(host.contains('.') && !host.starts_with('.') && !host.ends_with('.')) {
        return None;
    }
    // 手元のサーバーは TLS を使わないことが多いので http にする。それ以外は https
    let scheme = if local { "http" } else { "https" };
    Url::parse(&format!("{scheme}://{input}")).ok()
}

/// 手元のサーバー以外への `http:` を `https:` に直す
fn upgrade_to_https(mut url: Url) -> Url {
    if url.scheme() == "http" && !policy::is_loopback(&url) {
        // http から https への変更は失敗しない。番号は 80 のままにせず、既定に戻す
        if url.port() == Some(80) {
            let _ = url.set_port(None);
        }
        let _ = url.set_scheme("https");
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(input: &str) -> String {
        resolve(input).unwrap().to_string()
    }

    fn blocked(input: &str) -> Blocked {
        match resolve(input) {
            Err(Rejected::Blocked(reason)) => reason,
            other => panic!("{input:?} が断られなかった: {other:?}"),
        }
    }

    #[test]
    fn keeps_full_urls() {
        assert_eq!(resolved("https://servo.org/"), "https://servo.org/");
        assert_eq!(resolved("about:blank"), "about:blank");
    }

    #[test]
    fn upgrades_http_to_https() {
        assert_eq!(
            resolved("http://example.com/a?b=c"),
            "https://example.com/a?b=c"
        );
        assert_eq!(resolved("http://example.com:80/"), "https://example.com/");
        assert_eq!(
            resolved("http://example.com:8080/"),
            "https://example.com:8080/"
        );
    }

    #[test]
    fn adds_https_to_host_names() {
        assert_eq!(resolved("servo.org"), "https://servo.org/");
        assert_eq!(
            resolved("ja.wikipedia.org/wiki/Rust"),
            "https://ja.wikipedia.org/wiki/Rust"
        );
        assert_eq!(resolved("  example.com  "), "https://example.com/");
    }

    #[test]
    fn local_addresses_use_http() {
        assert_eq!(resolved("localhost:8000"), "http://localhost:8000/");
        assert_eq!(resolved("localhost"), "http://localhost/");
        assert_eq!(resolved("127.0.0.1:3000/x"), "http://127.0.0.1:3000/x");
        assert_eq!(resolved("http://localhost:8000/"), "http://localhost:8000/");
    }

    #[test]
    fn other_schemes_are_refused_not_searched() {
        assert_eq!(blocked("/tmp/a.html"), Blocked::Scheme);
        assert_eq!(blocked("file:///etc/passwd"), Blocked::Scheme);
        assert_eq!(blocked("javascript:alert(1)"), Blocked::Scheme);
        assert_eq!(blocked("ftp://example.com/"), Blocked::Scheme);
        assert_eq!(blocked("data:text/html,<h1>x</h1>"), Blocked::DataDocument);
    }

    #[test]
    fn other_input_is_searched() {
        assert_eq!(resolved("rust"), "https://duckduckgo.com/html/?q=rust");
        assert_eq!(
            resolved("servo browser engine"),
            "https://duckduckgo.com/html/?q=servo+browser+engine"
        );
        assert_eq!(
            resolved("日本語"),
            "https://duckduckgo.com/html/?q=%E6%97%A5%E6%9C%AC%E8%AA%9E"
        );
        assert_eq!(
            resolved("example.com is down"),
            "https://duckduckgo.com/html/?q=example.com+is+down"
        );
    }

    #[test]
    fn empty_input_opens_nothing() {
        assert_eq!(resolve(""), Err(Rejected::Empty));
        assert_eq!(resolve("   "), Err(Rejected::Empty));
    }
}
