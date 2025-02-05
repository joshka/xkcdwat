use axum::{
    extract::Request,
    http::{header, HeaderMap, StatusCode, Uri},
    middleware::{self, Next},
    response::{self, Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use axum_extra::extract::Host;
use tower_http::trace::{
    DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer,
};
use tracing::{error, info, Level};

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
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(DefaultOnFailure::new().level(Level::ERROR));
    let app = Router::new()
        .route("/", get(readme))
        .route("/feed", get(feed))
        .route_layer(middleware::from_fn(redirect_host_name))
        .layer(trace_layer);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

// replace the old host name with the current one (and keep the rest of the request the same)
async fn redirect_host_name(Host(host): Host, request: Request, next: Next) -> Response {
    if host.to_lowercase() == "xkcd-with-alt-text.joshka.net" {
        let uri = Uri::builder()
            .scheme("https")
            .authority("xkcdwat.joshka.net")
            .path_and_query(request.uri().path_and_query().unwrap().to_string())
            .build()
            .unwrap()
            .to_string();
        return Redirect::permanent(&uri).into_response();
    }

    next.run(request).await
}

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
        .map_err(|e| {
            error!("Failed to build request: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let xkcd_response = client.execute(request).await.map_err(|e| {
        error!("Failed to execute request: {}", e);
        StatusCode::BAD_GATEWAY
    })?;
    let content_type = &xkcd_response
        .headers()
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/rss+xml")
        .to_string();
    let feed = xkcd_response.text().await.map_err(|e| {
        error!("Failed to read response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let re = regex::Regex::new(r#"(&lt;img.*? alt="(?<alt>.*?)".*?&gt;)"#).unwrap();
    let feed = feed.replacen(
        "<title>xkcd.com</title>",
        "<title>xkcdwat (xkcd with alt-text)</title>",
        1,
    );
    let feed = re.replace_all(&feed, "\n$0\n&lt;p&gt;alt-text: $alt&lt;/p&gt;\n");
    let feed = feed.to_string();

    Ok(([(header::CONTENT_TYPE, content_type)], feed).into_response())
}
