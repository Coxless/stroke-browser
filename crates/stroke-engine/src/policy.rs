//! 読み込んでよい URL の規則。エンジンは、ページ本体も画像などの資源もリダイレクト先も、
//! 読み込む前にすべてここで確かめる。
//!
//! - `https:` だけを許す
//! - `http:` は、手元のサーバー（`localhost`、`127.0.0.0/8`、`::1`）だけ許す
//! - `data:` はページの中の資源（画像など）だけ許す。ページとして開くのは許さない
//! - `about:`（`about:blank` など）は許す
//! - それ以外（`file:`、`ftp:` など）は許さない

use url::{Host, Url};

/// 読み込みを止めた理由
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    /// 手元のサーバー以外への `http:`
    InsecureHttp,
    /// `data:` をページとして開こうとした
    DataDocument,
    /// 許していないスキーム
    Scheme,
}

impl Blocked {
    pub fn message(self) -> &'static str {
        match self {
            Blocked::InsecureHttp => "HTTPS でない（http: は手元のサーバーだけ）",
            Blocked::DataDocument => "data: の URL はページとして開かない",
            Blocked::Scheme => "開けるのは https: と http://localhost だけ",
        }
    }
}

/// `url` を読み込んでよいか。`is_document` はページ本体（メインフレーム）の読み込みか。
pub fn check(url: &Url, is_document: bool) -> Result<(), Blocked> {
    match url.scheme() {
        "https" | "about" => Ok(()),
        "http" if is_loopback(url) => Ok(()),
        "http" => Err(Blocked::InsecureHttp),
        "data" if is_document => Err(Blocked::DataDocument),
        "data" => Ok(()),
        _ => Err(Blocked::Scheme),
    }
}

/// 手元のサーバーか
pub fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => domain.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(url: &str) -> Result<(), Blocked> {
        check(&Url::parse(url).unwrap(), true)
    }

    fn resource(url: &str) -> Result<(), Blocked> {
        check(&Url::parse(url).unwrap(), false)
    }

    #[test]
    fn https_is_allowed() {
        assert_eq!(page("https://servo.org/"), Ok(()));
        assert_eq!(resource("https://example.com/a.png"), Ok(()));
    }

    #[test]
    fn http_only_to_loopback() {
        assert_eq!(page("http://example.com/"), Err(Blocked::InsecureHttp));
        assert_eq!(
            resource("http://example.com/a.png"),
            Err(Blocked::InsecureHttp)
        );
        assert_eq!(page("http://localhost:8000/"), Ok(()));
        assert_eq!(page("http://127.0.0.1/"), Ok(()));
        assert_eq!(page("http://127.1.2.3/"), Ok(()));
        assert_eq!(page("http://[::1]:8080/"), Ok(()));
        // localhost に見えるだけの名前は手元ではない
        assert_eq!(
            page("http://localhost.example.com/"),
            Err(Blocked::InsecureHttp)
        );
        assert_eq!(page("http://192.168.0.1/"), Err(Blocked::InsecureHttp));
    }

    #[test]
    fn data_only_as_resource() {
        assert_eq!(
            page("data:text/html,<h1>x</h1>"),
            Err(Blocked::DataDocument)
        );
        assert_eq!(resource("data:image/png;base64,AAAA"), Ok(()));
    }

    #[test]
    fn about_is_allowed() {
        assert_eq!(page("about:blank"), Ok(()));
    }

    #[test]
    fn other_schemes_are_blocked() {
        assert_eq!(page("file:///etc/passwd"), Err(Blocked::Scheme));
        assert_eq!(resource("file:///etc/passwd"), Err(Blocked::Scheme));
        assert_eq!(page("ftp://example.com/"), Err(Blocked::Scheme));
        assert_eq!(page("javascript:alert(1)"), Err(Blocked::Scheme));
        assert_eq!(resource("blob:https://example.com/x"), Err(Blocked::Scheme));
    }
}
