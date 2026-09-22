# xkcdwat (xkcd-with-alt-text)

A small web app to make viewing xkcd easier in a mobile feed reader.

The app reads xkcd.com/rss.xml and adds the alt-text from the images to the description.

To see this running see: <http://xkcdwat.joshka.net/feed>

Contribute at <https://github.com/joshka/xkcdwat>

## Development and CI

The project follows current stable Rust and does not promise a fixed minimum Rust version. Keep
`Cargo.lock` committed so local builds and CI use the same dependency versions.

Before opening a pull request, run:

```sh
cargo +nightly fmt -- --check
cargo +stable clippy --locked --all-features --all-targets
cargo +stable test --locked --all-features --all-targets
cargo +nightly doc --locked --no-deps --all-features
```

Formatting uses nightly because `rustfmt.toml` enables unstable options. CI runs Clippy and tests
once on stable Rust, then builds docs on nightly. Clippy warnings appear as annotations on pull
requests. The crate currently has no tests, so coverage reporting would add no useful signal.

The [CI workflow](.github/workflows/ci.yml) runs on pull requests and pushes to `main`. A push to
`main` deploys to Fly.io only after every CI job passes. Dependabot checks Cargo and GitHub Actions
dependencies weekly.
