//! URL 入力欄に打った文字を、開く URL に直す。

use std::net::Ipv4Addr;

use url::Url;

/// URL でない入力を渡す検索エンジン。`{}` に検索語が入る。設定で変えられるようにするのは Phase 6
const SEARCH_URL: &str = "https://duckduckgo.com/html/?q={}";

/// 入力を URL に直す。URL に見えなければ検索の URL にする。空なら `None`。
pub fn resolve(input: &str) -> Option<Url> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    if let Some(url) = as_url(input) {
        return Some(url);
    }
    let query: String = url::form_urlencoded::byte_serialize(input.as_bytes()).collect();
    Url::parse(&SEARCH_URL.replace("{}", &query)).ok()
}

fn as_url(input: &str) -> Option<Url> {
    // 空白を含むものは URL として打ったものではない
    if input.contains(char::is_whitespace) {
        return None;
    }
    // 手元のファイル
    if input.starts_with('/') {
        return Url::from_file_path(input).ok();
    }
    // スキームを書いたもの。`localhost:8000` のように「名前:番号」も URL として読めてしまうので、
    // ブラウザで開くスキームに限る
    if let Ok(url) = Url::parse(input)
        && matches!(url.scheme(), "http" | "https" | "file" | "about" | "data")
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
    // 手元のサーバーは TLS を使わないことが多い
    let scheme = if local { "http" } else { "https" };
    Url::parse(&format!("{scheme}://{input}")).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(input: &str) -> String {
        resolve(input).unwrap().to_string()
    }

    #[test]
    fn keeps_full_urls() {
        assert_eq!(resolved("https://servo.org/"), "https://servo.org/");
        assert_eq!(
            resolved("http://example.com/a?b=c"),
            "http://example.com/a?b=c"
        );
        assert_eq!(resolved("about:blank"), "about:blank");
        assert_eq!(resolved("file:///tmp/a.html"), "file:///tmp/a.html");
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
    }

    #[test]
    fn file_paths_become_file_urls() {
        assert_eq!(resolved("/tmp/a.html"), "file:///tmp/a.html");
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
        assert_eq!(resolve(""), None);
        assert_eq!(resolve("   "), None);
    }
}
