//! Make xkcd's image alt text readable in RSS clients that show descriptions but not image titles.
//!
//! `/feed` fetches xkcd's RSS and adds a visible paragraph beneath each image. The feed keeps its
//! escaped HTML so readers can render those paragraphs as part of each comic's description.
//! `/` serves the repository README, and the legacy hostname redirects existing subscriptions to
//! the canonical hostname. No feed content is stored between requests.

use pulldown_cmark::{html, Parser};
use regex::Regex;
use worker::{console_error, event, Context, Env, Fetch, Method, Request, Response, Result, Url};

const FEED_URL: &str = "https://xkcd.com/rss.xml";
const LEGACY_HOST: &str = "xkcd-with-alt-text.joshka.net";
const CANONICAL_HOST: &str = "xkcdwat.joshka.net";
const DEFAULT_CONTENT_TYPE: &str = "application/rss+xml";

/// Route the home page and feed, including subscriptions using the legacy hostname.
///
/// Redirects apply only to supported paths: an unknown URL returns 404 on either hostname.
#[event(fetch)]
async fn fetch(request: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let url = request.url()?;
    if !matches!(url.path(), "/" | "/feed") {
        return Response::error("Not found", 404);
    }

    // The request URL identifies a Custom Domain even when local Workers dev rewrites Host.
    let request_host = url.host_str();
    let is_legacy_host = request_host.is_some_and(|host| host.eq_ignore_ascii_case(LEGACY_HOST));
    if is_legacy_host {
        let redirect_url = legacy_redirect_url(&url)?;
        return Response::redirect_with_status(redirect_url, 308);
    }

    match (request.method(), url.path()) {
        (Method::Get | Method::Head, "/") => readme(),
        (Method::Get | Method::Head, "/feed") => feed(&request).await,
        _ => Response::error("Method not allowed", 405),
    }
}

/// Preserve reader bookmarks by changing only the origin of a legacy URL.
fn legacy_redirect_url(request_url: &Url) -> Result<Url> {
    let mut url = request_url.clone();
    url.set_scheme("https")
        .map_err(|()| worker::Error::RustError("invalid redirect scheme".into()))?;
    url.set_host(Some(CANONICAL_HOST))?;
    url.set_port(None)
        .map_err(|()| worker::Error::RustError("invalid redirect port".into()))?;
    Ok(url)
}

/// Use the repository README as the home page so project information has one source.
///
/// It is embedded at build time, so README edits require a rebuild.
fn readme() -> Result<Response> {
    let markdown = include_str!("../README.md");
    let parser = Parser::new(markdown);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    Response::from_html(html_output)
}

/// Pass the reader's `Accept` upstream and preserve xkcd's `Content-Type` downstream.
///
/// Upstream HTTP status is not propagated: any successfully read body is rewritten and returned
/// with 200, including error bodies. Transport failures return 502; body-read failures return 500.
async fn feed(request: &Request) -> Result<Response> {
    let accept = request
        .headers()
        .get("Accept")?
        .unwrap_or_else(|| DEFAULT_CONTENT_TYPE.into());

    let mut upstream_request = Request::new(FEED_URL, Method::Get)?;
    let upstream_headers = upstream_request.headers_mut()?;
    upstream_headers.set("Accept", &accept)?;
    let mut upstream = match Fetch::Request(upstream_request).send().await {
        Ok(response) => response,
        Err(error) => {
            console_error!("Failed to execute request: {error}");
            return Response::error("Failed to fetch xkcd feed", 502);
        }
    };

    let content_type = upstream
        .headers()
        .get("Content-Type")?
        .unwrap_or_else(|| DEFAULT_CONTENT_TYPE.into());
    let body = match upstream.text().await {
        Ok(body) => body,
        Err(error) => {
            console_error!("Failed to read response: {error}");
            return Response::error("Failed to read xkcd feed", 500);
        }
    };

    let rewritten_feed = rewrite_feed(&body);
    let mut response = Response::ok(rewritten_feed)?;
    response.headers_mut().set("Content-Type", &content_type)?;
    Ok(response)
}

/// Make image alt text visible in feed readers while preserving the surrounding RSS.
///
/// xkcd puts escaped HTML in RSS descriptions. Text substitution adds paragraphs without decoding
/// entities or reserializing unrelated markup. Only the first matching title is renamed to identify
/// this feed; subsequent titles are left alone. The image pattern depends on xkcd's escaped markup
/// and double-quoted alt attributes.
fn rewrite_feed(feed: &str) -> String {
    let feed = feed.replacen(
        "<title>xkcd.com</title>",
        "<title>xkcdwat (xkcd with alt-text)</title>",
        1,
    );
    let image = Regex::new(r#"(&lt;img.*? alt="(?<alt>.*?)".*?&gt;)"#).unwrap();
    image
        .replace_all(&feed, "\n$0\n&lt;p&gt;alt-text: $alt&lt;/p&gt;\n")
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_title_once_and_each_escaped_image() {
        let feed = concat!(
            "<title>xkcd.com</title><title>xkcd.com</title>",
            "&lt;img src=\"one.png\" alt=\"first\"&gt;",
            "&lt;img src=\"two.png\" alt=\"second\"&gt;"
        );
        let expected = concat!(
            "<title>xkcdwat (xkcd with alt-text)</title><title>xkcd.com</title>",
            "\n&lt;img src=\"one.png\" alt=\"first\"&gt;\n",
            "&lt;p&gt;alt-text: first&lt;/p&gt;\n",
            "\n&lt;img src=\"two.png\" alt=\"second\"&gt;\n",
            "&lt;p&gt;alt-text: second&lt;/p&gt;\n"
        );

        assert_eq!(rewrite_feed(feed), expected);
    }

    #[test]
    fn preserves_query_when_redirecting_legacy_host() {
        let request_url = "http://xkcd-with-alt-text.joshka.net:8912/feed?source=reader";
        let url = Url::parse(request_url).unwrap();
        let redirect = legacy_redirect_url(&url).unwrap();

        assert_eq!(
            redirect.as_str(),
            "https://xkcdwat.joshka.net/feed?source=reader"
        );
    }
}
