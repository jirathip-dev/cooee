// web/ts/md.ts, the renderer of the agent's replies (#43): its HTML for each element, escaping by construction
// (raw HTML and script stay text), links of http and https only, and how much of a streaming reply is settled.
import { expect, test } from "@playwright/test";

test("md.ts renders each element as DOM, escaping all text", async ({ page }) => {
  await page.goto("/pair");
  const out = await page.evaluate(async () => {
    const { render, settled } = await import("/md.js");
    const html = (s) => {
      const d = document.createElement("div");
      d.append(render(s));
      return d.innerHTML;
    };
    return {
      escaped: html('<script>alert(1)</script> & <b>x</b> <img src=x onerror=alert(1)> "q"'),
      table: html("| a | b | c |\n|:--|--:|:-:|\n| 1 | **2** | `3` |\n| x \\| y | | z |"),
      fence: html("```rust\nfn a() {\n  <b>1</b>\n\n}\n```"),
      links: html("[x](https://x.dev/a?b=1&c=2) [bad](javascript:alert(1)) [ftp](ftp://e.com) see https://e.com/p. ![i](https://e.com/i.png)"),
      inline: html("# Title\n\nSome *em*, **strong**, _u_, snake_case_name and `a <b>`.\nline two"),
      lists: html("- a\n- b\n  - c\n\n3. three\n4. four\n\nthen\n\n1. loose\n\n2. list"),
      quote: html("> quoted **x**\n> more\n\n---"),
      settled: [settled("a\n\nb"), settled("```\nx\n\ny"), settled("| a | b |\n|---|---|\n| 1"), settled("```\nx\n```\n\nmore")],
    };
  });
  expect(out.escaped).toBe(
    '<p>&lt;script&gt;alert(1)&lt;/script&gt; &amp; &lt;b&gt;x&lt;/b&gt; &lt;img src=x onerror=alert(1)&gt; "q"</p>',
  );
  expect(out.table).toBe(
    '<div class="tbl"><table><thead><tr><th style="text-align: left;">a</th><th style="text-align: right;">b</th><th style="text-align: center;">c</th></tr></thead>' +
      '<tbody><tr><td style="text-align: left;">1</td><td style="text-align: right;"><strong>2</strong></td><td style="text-align: center;"><code>3</code></td></tr>' +
      '<tr><td style="text-align: left;">x | y</td><td style="text-align: right;"></td><td style="text-align: center;">z</td></tr></tbody></table></div>',
  );
  expect(out.fence).toBe(
    '<div class="cb"><div class="cbh"><span>rust</span><button class="cp" type="button" aria-label="copy the rust code" data-copy="fn a() {\n  &lt;b&gt;1&lt;/b&gt;\n\n}">copy</button></div>' +
      "<pre><code>fn a() {\n  &lt;b&gt;1&lt;/b&gt;\n\n}</code></pre></div>",
  );
  const a = (href, text) => `<a href="${href}" target="_blank" rel="noopener">${text}</a>`;
  expect(out.links).toBe(
    `<p>${a("https://x.dev/a?b=1&amp;c=2", "x")} [bad](javascript:alert(1)) [ftp](ftp://e.com) see ${a("https://e.com/p", "https://e.com/p")}. ` +
      `![i](${a("https://e.com/i.png", "https://e.com/i.png")})</p>`,
  );
  expect(out.inline).toBe(
    "<h1>Title</h1><p>Some <em>em</em>, <strong>strong</strong>, <em>u</em>, snake_case_name and <code>a &lt;b&gt;</code>.<br>line two</p>",
  );
  expect(out.lists).toBe(
    '<ul><li>a</li><li>b<ul><li>c</li></ul></li></ul><ol start="3"><li>three</li><li>four</li></ol><p>then</p><ol><li><p>loose</p></li><li><p>list</p></li></ol>',
  );
  expect(out.quote).toBe("<blockquote><p>quoted <strong>x</strong><br>more</p></blockquote><hr>");
  // Up to the last blank line; nothing inside an open fence; a half-typed table waits.
  expect(out.settled).toEqual([3, 0, 0, 11]);
});

test("a fenced block gets [▸ terminal] beside [copy] only where the owner terminal's tab is on (#29)", async ({ page }) => {
  const fence = async () =>
    page.evaluate(async () => {
      const { render } = await import("/md.js");
      const d = document.createElement("div");
      d.append(render("```sh\ncd ~\nls -la\n```"));
      return [...d.querySelectorAll("button")].map((b) => [b.textContent, b.dataset.copy ?? b.dataset.term]);
    });
  // Pairing has no tabs: [copy] only (as in the test above).
  await page.goto("/pair");
  expect(await fence()).toEqual([["copy", "cd ~\nls -la"]]);
  // The chats, with the Terminal tab: [▸ terminal] carries the block's exact text.
  await page.goto("/more");
  await expect(page.locator('.gr[href="/p/terminal"]')).toContainText("Terminal");
  await page.goto("/");
  expect(await fence()).toEqual([
    ["copy", "cd ~\nls -la"],
    ["▸ terminal", "cd ~\nls -la"],
  ]);
});
