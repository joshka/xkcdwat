use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{self, Html, IntoResponse},
    routing::get,
    Router,
};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{info, Level};

async fn readme() -> Html<String> {
    let readme = include_str!("../README.md");
    let parser = pulldown_cmark::Parser::new(readme);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output.into()
}

async fn feed(headers: HeaderMap) -> response::Result<impl IntoResponse> {
    let client = reqwest::Client::new();
    let content_type = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/rss+xml");
    let request = client
        .get("https://xkcd.com/rss.xml")
        .header(reqwest::header::ACCEPT, content_type)
        .build()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let xkcd_response = client
        .execute(request)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;
    let content_type = &xkcd_response
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/rss+xml")
        .to_string();
    let feed = xkcd_response
        .text()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let feed = feed.replace("xkcd.com", "xkcd.com - with alt-text");

    let re = regex::Regex::new(r#"(&lt;img.*? alt="(?<alt>.*?)".*?&gt;)"#).unwrap();
    let feed = re.replace_all(&feed, "\n$0\n&lt;p&gt;alt-text: $alt&lt;/p&gt;\n");

    let response = ([(header::CONTENT_TYPE, content_type)], feed.to_string()).into_response();
    Ok(response)
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    let include_headers = DefaultMakeSpan::new()
        .level(Level::INFO)
        .include_headers(true);
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(include_headers)
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO));
    let app = Router::new()
        .route("/", get(readme))
        .route("/feed", get(feed))
        .layer(trace_layer);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
