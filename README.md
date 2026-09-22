# xkcdwat (xkcd-with-alt-text)

A small web app to make viewing xkcd easier in a mobile feed reader.

The app reads xkcd.com/rss.xml and adds the image alt text to each feed description. The live feed
is at <https://xkcdwat.joshka.net/feed>.

Contribute at <https://github.com/joshka/xkcdwat>.

## Behavior

`GET /feed` forwards the reader's `Accept` header to xkcd, rewrites the first feed title and the
escaped image markup, and returns xkcd's content type. Requests for `/` and `/feed` on the legacy
`xkcd-with-alt-text.joshka.net` hostname permanently redirect to the same path and query on
`xkcdwat.joshka.net`. `GET /` renders this README as HTML. Unknown paths return 404 on either hostname.

## Development

Install current stable Rust, nightly rustfmt, the `wasm32-unknown-unknown` target, the `worker-build`
0.8 series, and Node.js 24 or newer. The Worker is written in Rust; Wrangler provides the local
runtime and deployment CLI.

```sh
rustup target add wasm32-unknown-unknown
cargo install worker-build --version '^0.8' --locked
npm ci
npm run dev
```

Before opening a pull request, run the same checks as CI:

```sh
cargo +nightly fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
npm run check:deploy
npm test
```

## CI

The [CI workflow](https://github.com/joshka/xkcdwat/blob/main/.github/workflows/ci.yml) runs on
pull requests and pushes to `main`:

- **Format** checks `rustfmt.toml` with nightly Rust because that file uses unstable options.
- **Verify** runs tests and Clippy on stable Rust, then uses Wrangler's dry run to build and
  validate the WebAssembly bundle without deploying it. A local workerd smoke test checks the
  home page, redirects, and HTTP errors without contacting xkcd.

Cloudflare Workers Builds deploys pushes to `main` after the repository is connected. It runs
independently of GitHub Actions, so require Format and Verify on pull requests before merging if
those checks must gate deployment.

Dependabot checks Cargo, npm, and GitHub Actions dependencies weekly. Routine version updates have
a seven-day cooldown; npm and Cargo major releases have a 30-day cooldown. Security updates are
unaffected.

## Deployment

Workers Builds uses its own build image, which does not include Rust. Its build command runs
`npm run build` to install stable Rust, the Wasm target, and `worker-build` before compiling the
Worker. Its deploy command runs `npm run deploy`, which adds Rust to `PATH` before Wrangler's
custom build. Cloudflare's Git integration manages the deploy credential; the GitHub workflow
does not need Cloudflare secrets.

For initial setup:

1. Deploy `xkcdwat-preview` with `npm run deploy:preview` and verify its `workers.dev` URL.
1. Open a pull request, wait for its Format and Verify checks in GitHub Actions to pass, then
   merge it into `main` before connecting automatic deployment.
1. Remove any existing CNAME records for the two hostnames in Cloudflare DNS, then run
   `npm run deploy` locally to attach them to the `xkcdwat` Worker. This one-time deploy avoids
   waiting for the first hosted Rust build during the DNS cutover.
1. Connect the existing `xkcdwat` Worker to this repository in Cloudflare Workers & Pages. Grant
   the Cloudflare GitHub app access to this repository. Select `main` as the production branch,
   set **Build command** to `npm run build`, and set **Deploy command** to `npm run deploy`.
1. Check the first Workers Build and subsequent pushes to `main`.
1. Check `/`, `/feed`, and the legacy redirect on the production hostnames.

Cloudflare cannot attach a Worker Custom Domain while a CNAME exists for that hostname. The
preview Worker makes it possible to verify the code before changing DNS. The GitHub connection
and DNS cutover require account access. Cloudflare's Builds API can configure triggers, but
`wrangler login` does not grant that API's separate Workers Builds Configuration permission; use
a user-scoped API token for API-based setup.
