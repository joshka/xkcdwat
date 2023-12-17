use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{self, Html, IntoResponse, Response},
    routing::get,
    Router,
};

async fn readme() -> Html<String> {
    let readme = include_str!("../README.md");
    let parser = pulldown_cmark::Parser::new(readme);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output.into()
}

async fn feed(headers: HeaderMap) -> response::Result<impl IntoResponse, StatusCode> {
    let client = reqwest::Client::new();
    let content_type = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/rss+xml");
    let request = client
        .get("https://xkcd.com/rss.xml")
        .header(reqwest::header::ACCEPT, content_type)
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let xkcd_response = client
        .execute(request)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    let content_type = &xkcd_response
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/rss+xml")
        .to_string();
    let feed = xkcd_response
        .text()
        .await
        .map_err(|_e| StatusCode::BAD_GATEWAY)?;

    let feed = feed.replace("xkcd.com", "xkcd.com - with alt-text");

    let re = regex::Regex::new(r#"(&lt;img.*? alt="(?<alt>.*?)".*?&gt;)"#).unwrap();
    let feed = re.replace_all(&feed, "\n$0\n&lt;p&gt;alt-text: $alt&lt;/p&gt;\n");
    let response = Response::builder()
        .header("Content-Type", content_type)
        .body(feed.to_string())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(response)
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(readme))
        .route("/feed", get(feed));

    Ok(router.into())
}
