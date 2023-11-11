use axum::{
    http::{self, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use reqwest::Client;

async fn readme() -> axum::response::Html<String> {
    let readme = include_str!("../README.md");
    let parser = pulldown_cmark::Parser::new(readme);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output.into()
}

async fn feed(headers: HeaderMap) -> axum::response::Result<impl IntoResponse, http::StatusCode> {
    // this is a mess - I'm sure there's a much neater way, but I mostly let Copilot write this
    let content_type = headers
        .get("Accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/rss+xml");
    let client = Client::new();
    client
        .get("https://xkcd.com/rss.xml")
        .header("Accept", content_type)
        .build()
        .map_err(|_e| http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let xkcd_response = reqwest::get("https://xkcd.com/rss.xml")
        .await
        .map_err(|_e| http::StatusCode::BAD_GATEWAY)?;
    let content_type = &xkcd_response
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/rss+xml")
        .to_string();
    let feed = xkcd_response
        .text()
        .await
        .map_err(|_e| http::StatusCode::BAD_GATEWAY)?;

    let feed = feed.replace("xkcd.com", "xkcd.com - with alt-text");

    let re = regex::Regex::new(r#"(&lt;img.*? alt="(?<alt>.*?)".*?&gt;)"#).unwrap();
    let feed = re.replace_all(&feed, "\n$0\n&lt;p&gt;alt-text: $alt&lt;/p&gt;\n");
    let response = Response::builder()
        .header("Content-Type", content_type)
        .body(feed.to_string())
        .map_err(|_e| http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(response)
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(readme))
        .route("/feed", get(feed));

    Ok(router.into())
}
