// Exercise the compiled Rust handler in workerd: native Rust tests cannot catch Wasm or HTTP
// integration failures. These routes need no upstream request, so CI does not depend on xkcd.
import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { createTestHarness } from "wrangler";

const server = createTestHarness({
  workers: [{ configPath: "./wrangler.jsonc" }],
});

before(() => server.listen());
after(() => server.close());

test("home page renders the README", async () => {
  const response = await server.fetch("https://xkcdwat.joshka.net/");
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type"), /text\/html/);
  assert.match(await response.text(), /<h1>xkcdwat/);
});

test("legacy subscriptions redirect with their path and query", async () => {
  const response = await server.fetch(
    "https://xkcd-with-alt-text.joshka.net/feed?source=reader",
    { redirect: "manual" },
  );
  assert.equal(response.status, 308);
  assert.equal(response.headers.get("location"), "https://xkcdwat.joshka.net/feed?source=reader");
});

test("unknown paths return 404 on either hostname", async () => {
  for (const host of ["xkcdwat.joshka.net", "xkcd-with-alt-text.joshka.net"]) {
    const response = await server.fetch(`https://${host}/missing`, { redirect: "manual" });
    assert.equal(response.status, 404);
  }
});

test("unsupported methods return 405", async () => {
  const response = await server.fetch("https://xkcdwat.joshka.net/feed", { method: "POST" });
  assert.equal(response.status, 405);
});
