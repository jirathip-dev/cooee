# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: look.spec.mjs >> switching style or figures keeps the scroll position
- Location: look.spec.mjs:260:1

# Error details

```
Error: style terminal

expect(received).toBeLessThanOrEqual(expected)

Expected: <= 1
Received:    1.106808875608676
```

# Page snapshot

```yaml
- generic [ref=e1]:
  - generic [ref=e2]:
    - banner [ref=e3]:
      - link "cooee" [ref=e4] [cursor=pointer]:
        - /url: /
      - button "refresh" [ref=e5] [cursor=pointer]: "[ ↻ ]"
      - link "new chat" [ref=e6] [cursor=pointer]:
        - /url: /new?agent=fake-acp
        - text: "[ + ]"
      - 'button "look: theme and colours" [pressed] [ref=e7] [cursor=pointer]': "[ ◐ ]"
      - button "settings · alerts are off" [ref=e8] [cursor=pointer]: "[ ⚙ ]"
    - main [ref=e9]:
      - text: "[] [] [] [] [] [] [] [] [] [] [] []"
      - generic [ref=e10]:
        - generic [ref=e11]:
          - link "back to agents" [ref=e12] [cursor=pointer]:
            - /url: /?list&agent=fake-acp
            - text: "[ ‹ ]"
          - generic [aria-hidden] [ref=e13]: FA
          - generic [ref=e14]:
            - generic [ref=e15]:
              - heading "Fake-390" [level=1] [ref=e16]
              - generic [ref=e17]: long-390
            - generic [ref=e18]: Plain · fake-390 · main login · ~/root/project ·
          - button "close the thread" [ref=e20] [cursor=pointer]: "[ × ]"
        - text: "[]"
        - list "messages" [ref=e21]:
          - listitem [ref=e22]:
            - generic [ref=e23]: Fake-390
            - paragraph [ref=e25]: "#966 ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e26]:
            - generic [ref=e27]: "#967 review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e28]: 09:20 PM ✓
          - listitem [ref=e29]:
            - generic [ref=e30]: Fake-390
            - paragraph [ref=e32]: "#968 found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e33]:
            - generic [ref=e34]: "#969 two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e35]: 09:22 PM ✓
          - listitem [ref=e36]:
            - generic [ref=e37]: Fake-390
            - paragraph [ref=e39]: "#970 issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e40]:
            - generic [ref=e41]: "#971 merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e42]: 09:24 PM ✓
          - listitem [ref=e43]:
            - generic [ref=e44]: Fake-390
            - paragraph [ref=e46]: "#972 the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e47]:
            - generic [ref=e48]: "#973 build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e49]: 09:26 PM ✓
          - listitem [ref=e50]:
            - generic [ref=e51]: Fake-390
            - paragraph [ref=e53]: "#974 passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e54]:
            - generic [ref=e55]: "#975 on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e56]: 09:28 PM ✓
          - listitem [ref=e57]:
            - generic [ref=e58]: Fake-390
            - paragraph [ref=e60]: "#976 staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e61]:
            - generic [ref=e62]: "#977 สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e63]: 09:30 PM ✓
          - listitem [ref=e64]:
            - generic [ref=e65]: Fake-390
            - paragraph [ref=e67]: "#978 ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e68]:
            - generic [ref=e69]: "#979 review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e70]: 09:32 PM ✓
          - listitem [ref=e71]:
            - generic [ref=e72]: Fake-390
            - paragraph [ref=e74]: "#980 found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e75]:
            - generic [ref=e76]: "#981 two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e77]: 09:34 PM ✓
          - listitem [ref=e78]:
            - generic [ref=e79]: Fake-390
            - paragraph [ref=e81]: "#982 issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e82]:
            - generic [ref=e83]: "#983 merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e84]: 09:36 PM ✓
          - listitem [ref=e85]:
            - generic [ref=e86]: Fake-390
            - paragraph [ref=e88]: "#984 the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e89]:
            - generic [ref=e90]: "#985 build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e91]: 09:38 PM ✓
          - listitem [ref=e92]:
            - generic [ref=e93]: Fake-390
            - paragraph [ref=e95]: "#986 passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e96]:
            - generic [ref=e97]: "#987 on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e98]: 09:40 PM ✓
          - listitem [ref=e99]:
            - generic [ref=e100]: Fake-390
            - paragraph [ref=e102]: "#988 staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e103]:
            - generic [ref=e104]: "#989 สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e105]: 09:42 PM ✓
          - listitem [ref=e106]:
            - generic [ref=e107]: Fake-390
            - paragraph [ref=e109]: "#990 ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e110]:
            - generic [ref=e111]: "#991 review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e112]: 09:44 PM ✓
          - listitem [ref=e113]:
            - generic [ref=e114]: Fake-390
            - paragraph [ref=e116]: "#992 found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e117]:
            - generic [ref=e118]: "#993 two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e119]: 09:46 PM ✓
          - listitem [ref=e120]:
            - generic [ref=e121]: Fake-390
            - paragraph [ref=e123]: "#994 issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e124]:
            - generic [ref=e125]: "#995 merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e126]: 09:48 PM ✓
          - listitem [ref=e127]: unread
          - listitem [ref=e128]:
            - generic [ref=e129]: Fake-390
            - paragraph [ref=e131]: "#996 the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e132]:
            - generic [ref=e133]: "#997 build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e134]: 09:50 PM ✓
          - listitem [ref=e135]:
            - generic [ref=e136]: Fake-390
            - paragraph [ref=e138]: "#998 passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
          - listitem [ref=e139]:
            - generic [ref=e140]: "#999 on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on"
            - generic [ref=e141]: 09:52 PM ✓
          - listitem [ref=e142]:
            - generic [ref=e143]: Fake-390
            - paragraph [ref=e145]: "#1000 staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two staging merge ที่นี่ build found on issues สวัสดีครับ the review passed two"
        - generic [ref=e146]:
          - 'button "[folder: project ▾]" [ref=e147] [cursor=pointer]'
          - text: "[ ] ✓ []"
          - combobox "model" [ref=e148]:
            - 'option "model: default" [selected]'
            - 'option "model: opus"'
            - 'option "model: sonnet"'
            - 'option "model: opus-2"'
            - 'option "model: sonnet-2"'
          - combobox "effort" [ref=e149]:
            - 'option "effort: default" [selected]'
            - 'option "effort: low"'
            - 'option "effort: high"'
        - generic [ref=e150]:
          - text: "[]"
          - generic [ref=e151]:
            - generic [ref=e152] [cursor=pointer]:
              - text: "[ 📎"
              - button "attach files" [ref=e153]
              - text: "]"
            - textbox "message" [ref=e154]:
              - /placeholder: message Fake-390
            - button "[send]" [ref=e155] [cursor=pointer]
        - text: "[] [] []"
  - generic:
    - dialog "look" [ref=e156]:
      - generic [ref=e157]:
        - generic [ref=e158]: look
        - button "close" [ref=e159] [cursor=pointer]: "[ × ]"
      - generic [ref=e160]:
        - generic [ref=e161]:
          - generic [ref=e162]: style
          - radiogroup "style" [ref=e163]:
            - radio "[modern]" [ref=e164] [cursor=pointer]
            - radio "[terminal]" [checked] [active] [ref=e165] [cursor=pointer]
            - radio "[scene]" [ref=e166] [cursor=pointer]
        - generic [ref=e167]:
          - generic [ref=e168]: figures
          - radiogroup "figures" [ref=e169]:
            - radio "[sprite]" [checked] [ref=e170] [cursor=pointer]
            - radio "[ascii]" [ref=e171] [cursor=pointer]
            - radio "[none]" [ref=e172] [cursor=pointer]
        - paragraph [ref=e173]: theme
        - listbox "theme" [ref=e174]:
          - option "farm ✓" [selected] [ref=e175] [cursor=pointer]:
            - generic [ref=e176]: farm
            - generic [ref=e177]: ✓
          - option "kitchen" [ref=e178] [cursor=pointer]
          - option "garden" [ref=e180] [cursor=pointer]
          - option "plain" [ref=e182] [cursor=pointer]
        - paragraph [ref=e184]: appearance
        - radiogroup "mode" [ref=e185]:
          - radio "[system]" [checked] [ref=e186] [cursor=pointer]
          - radio "[light]" [ref=e187] [cursor=pointer]
          - radio "[dark]" [ref=e188] [cursor=pointer]
        - paragraph [ref=e189]: dark colours · showing
        - listbox "dark colours" [ref=e190]:
          - option "Aa github" [ref=e191] [cursor=pointer]:
            - generic [ref=e192]: Aa
            - generic [ref=e197]: github
          - option "Aa solarized" [ref=e198] [cursor=pointer]:
            - generic [ref=e199]: Aa
            - generic [ref=e204]: solarized
          - option "Aa gruvbox" [ref=e205] [cursor=pointer]:
            - generic [ref=e206]: Aa
            - generic [ref=e211]: gruvbox
          - option "Aa nord" [ref=e212] [cursor=pointer]:
            - generic [ref=e213]: Aa
            - generic [ref=e218]: nord
          - option "Aa dracula" [ref=e219] [cursor=pointer]:
            - generic [ref=e220]: Aa
            - generic [ref=e225]: dracula
          - option "Aa farm default ✓" [selected] [ref=e226] [cursor=pointer]:
            - generic [ref=e227]: Aa
            - generic [ref=e232]: farm
            - generic [ref=e233]: default
            - generic [ref=e234]: ✓
          - option "Aa kitchen" [ref=e235] [cursor=pointer]:
            - generic [ref=e236]: Aa
            - generic [ref=e241]: kitchen
          - option "Aa garden" [ref=e242] [cursor=pointer]:
            - generic [ref=e243]: Aa
            - generic [ref=e248]: garden
          - option "Aa sea" [ref=e249] [cursor=pointer]:
            - generic [ref=e250]: Aa
            - generic [ref=e255]: sea
        - paragraph [ref=e256]: light colours
        - listbox "light colours" [ref=e257]:
          - option "Aa paper" [ref=e258] [cursor=pointer]:
            - generic [ref=e259]: Aa
            - generic [ref=e264]: paper
          - option "Aa farm default ✓" [selected] [ref=e265] [cursor=pointer]:
            - generic [ref=e266]: Aa
            - generic [ref=e271]: farm
            - generic [ref=e272]: default
            - generic [ref=e273]: ✓
          - option "Aa kitchen" [ref=e274] [cursor=pointer]:
            - generic [ref=e275]: Aa
            - generic [ref=e280]: kitchen
          - option "Aa garden" [ref=e281] [cursor=pointer]:
            - generic [ref=e282]: Aa
            - generic [ref=e287]: garden
        - paragraph [ref=e288]: this device only · iOS keeps the icon it installed with
    - text: "[ ] [] [] [] [] [] [] [] [] [] [] [] [ ] [ ] [] [] [] [] [] [] [] [] [] [] [ ]"
```

# Test source

```ts
  188 |     await shot(page, info, `${name}-chat`);
  189 |   }
  190 | });
  191 | 
  192 | test("the ascii figures' two frames alternate", async ({ page, context, baseURL }) => {
  193 |   await page.emulateMedia({ colorScheme: "dark", reducedMotion: "no-preference" });
  194 |   await pickOn(context, baseURL, { figures: "ascii" });
  195 |   await page.goto("/");
  196 |   const fig = page.locator(".tp .rsa").first();
  197 |   await expect(fig.locator("b")).toHaveCount(2);
  198 |   // The frames' opacity at a time on the loop's clock (its animation paused there, so no real time passes).
  199 |   const at = (ms) => fig.evaluate((e, ms) => {
  200 |     for (const a of e.getAnimations()) { a.pause(); a.currentTime = ms; }
  201 |     return [...e.children].map((b) => getComputedStyle(b).opacity);
  202 |   }, ms);
  203 |   expect(await at(100)).toEqual(["1", "0"]);
  204 |   expect(await at(700)).toEqual(["0", "1"]);
  205 |   expect(await at(1300)).toEqual(["1", "0"]);
  206 |   // Reduced motion: the first frame, still.
  207 |   await page.emulateMedia({ reducedMotion: "reduce" });
  208 |   expect(await fig.evaluate((e) => e.getAnimations().length)).toBe(0);
  209 |   expect(await fig.evaluate((e) => [...e.children].map((b) => getComputedStyle(b).opacity))).toEqual(["1", "0"]);
  210 | });
  211 | 
  212 | test("the look sheet lists Scene only with sprites, and plain only ASCII or None", async ({ page }, info) => {
  213 |   await page.goto("/");
  214 |   await page.getByRole("button", { name: "look: theme and colours" }).click();
  215 |   const radios = (k) => sheet(page).getByRole("radiogroup", { name: k }).getByRole("radio");
  216 |   await expect(radios("style")).toHaveText(["modern", "terminal", "scene"]);
  217 |   await expect(radios("figures")).toHaveText(["sprite", "ascii", "none"]);
  218 |   // Scene in or out redraws the agents sidebar (#34, #86), which closes the sheet: open it again once the list is drawn.
  219 |   const again = async () => {
  220 |     if (!(await sheet(page).count())) await page.getByRole("button", { name: "look: theme and colours" }).click();
  221 |   };
  222 |   await tap(page, "style", "terminal");
  223 |   await tap(page, "style", "scene");
  224 |   await expect(page.locator("#scene")).toBeVisible();
  225 |   // Scene changes only the sidebar: pages keep the Terminal pick's shape.
  226 |   await expect(page.locator("html")).toHaveAttribute("data-style", "terminal");
  227 |   // ASCII or None hides Scene, and the device goes back to its Terminal pick.
  228 |   for (const f of ["ascii", "none"]) {
  229 |     await again();
  230 |     await tap(page, "figures", f);
  231 |     await expect(page.locator("#scene")).toHaveCount(0);
  232 |     await again();
  233 |     await expect(radios("style")).toHaveText(["modern", "terminal"]);
  234 |     await expect(page.locator("html")).toHaveAttribute("data-style", "terminal");
  235 |     await expect(sheet(page).locator('[data-k=style][data-v=terminal]')).toHaveAttribute("aria-checked", "true");
  236 |   }
  237 |   // plain: no sprites, so Figures offers ASCII or None, and there is no Scene; it starts on ASCII.
  238 |   await tap(page, "figures", "sprite");
  239 |   await expect(page.locator("#scene")).toBeVisible();
  240 |   await again();
  241 |   await expect(radios("style")).toHaveText(["modern", "terminal", "scene"]);
  242 |   await tap(page, "theme", "plain");
  243 |   await expect(page.locator("html")).toHaveAttribute("data-theme", "plain");
  244 |   await expect(page.locator("#scene")).toHaveCount(0);
  245 |   await again();
  246 |   await expect(radios("figures")).toHaveText(["ascii", "none"]);
  247 |   await expect(radios("style")).toHaveText(["modern", "terminal"]);
  248 |   await expect(page.locator("html")).toHaveAttribute("data-figures", "ascii");
  249 |   await expect(sheet(page).locator('[data-k=figures][data-v=ascii]')).toHaveAttribute("aria-checked", "true");
  250 |   await expect(page.locator(".tp .rsa").first()).toBeVisible();
  251 |   await shot(page, info, "look-plain-ascii");
  252 |   await audit(page);
  253 |   // The page drawn for it lists the same.
  254 |   await page.reload();
  255 |   await page.getByRole("button", { name: "look: theme and colours" }).click();
  256 |   await expect(radios("figures")).toHaveText(["ascii", "none"]);
  257 |   await expect(radios("style")).toHaveText(["modern", "terminal"]);
  258 | });
  259 | 
  260 | test("switching style or figures keeps the scroll position", async ({ page }, info) => {
  261 |   await page.goto(`/t/long-${info.project.name}`);
  262 |   await expect(page.locator("#msgs .unread")).toBeVisible();
  263 |   const list = page.locator("#msgs");
  264 |   // Mid-thread, with the top row cut through its middle.
  265 |   await list.evaluate((l) => {
  266 |     l.scrollTop = l.scrollHeight / 2;
  267 |     const r = [...l.children].find((r) => r.getBoundingClientRect().bottom > l.getBoundingClientRect().top);
  268 |     l.scrollTop += r.getBoundingClientRect().top - l.getBoundingClientRect().top + r.offsetHeight / 2;
  269 |   });
  270 |   // The first row in view, and how much of it is above the list's top edge (its share of the row's height): a
  271 |   // switch restyles every row, so the same row stays first in view, cut at the same share to the pixel (scrollTop
  272 |   // snaps to whole pixels, so a short row's share may move by a pixel's worth).
  273 |   const view = () => list.evaluate((l) => {
  274 |     const top = l.getBoundingClientRect().top, r = [...l.children].find((r) => r.getBoundingClientRect().bottom > top);
  275 |     const b = r.getBoundingClientRect();
  276 |     return { id: r.dataset.id, cut: (top - b.top) / b.height, h: b.height };
  277 |   });
  278 |   const before = await view();
  279 |   expect(before.cut).toBeGreaterThan(0);
  280 |   await page.getByRole("button", { name: "look: theme and colours" }).click();
  281 |   for (const [k, v] of [["style", "terminal"], ["figures", "none"], ["style", "modern"], ["figures", "sprite"]]) {
  282 |     const height = await list.evaluate((l) => l.scrollHeight);
  283 |     await tap(page, k, v);
  284 |     // The switch did move the rows: the thread's height changed.
  285 |     if (k === "style") expect(await list.evaluate((l) => l.scrollHeight)).not.toBe(height);
  286 |     const now = await view();
  287 |     expect(now.id, `${k} ${v}`).toBe(before.id);
> 288 |     expect(Math.abs(now.cut - before.cut) * now.h, `${k} ${v}`).toBeLessThanOrEqual(1);
      |                                                                 ^ Error: style terminal
  289 |   }
  290 | });
  291 | 
  292 | test("a plugin tab gets the theme's variables, data URLs, style and figures, and every change", async ({ page, request }) => {
  293 |   await page.goto("/");
  294 |   // A plugin tab as M4 will mount one: it asks for the look and keeps every answer.
  295 |   await page.evaluate(() => {
  296 |     const f = document.createElement("iframe");
  297 |     f.dataset.plugin = "drover";
  298 |     f.title = "drover";
  299 |     f.srcdoc = `<script>window.got = []; addEventListener("message", (e) => got.push(e.data)); parent.postMessage({ type: "cooee-look?" }, "*");<\/script>`;
  300 |     document.body.append(f);
  301 |   });
  302 |   const tab = page.frameLocator("iframe[data-plugin]");
  303 |   const got = () => page.frames().find((f) => f !== page.mainFrame()).evaluate(() => window.got);
  304 |   await expect.poll(async () => (await got())?.length).toBe(1);
  305 |   const [look] = await got();
  306 |   expect(look).toMatchObject({ type: "cooee-look", theme: "farm", mode: "dark", style: "modern", figures: "sprite" });
  307 |   expect(look).not.toHaveProperty("sprites");
  308 |   expect(look.vars["--bg"]).toBe("#0f1a13");
  309 |   expect(look.words.needs).toBe("at the gate");
  310 |   for (const [k, type] of [["sprites", "image/svg+xml"], ["icon", "image/png"], ["theme", "application/json"]]) {
  311 |     const r = await request.get(look.urls[k]);
  312 |     expect(r.ok(), k).toBe(true);
  313 |     expect(r.headers()["content-type"]).toBe(type);
  314 |   }
  315 |   expect(await (await request.get(look.urls.theme)).json()).toMatchObject({ label: "farm", words: { needs: "at the gate" } });
  316 |   expect(tab).toBeTruthy();
  317 |   await page.getByRole("button", { name: "look: theme and colours" }).click();
  318 |   await tap(page, "style", "terminal");
  319 |   await expect.poll(async () => (await got()).at(-1).style).toBe("terminal");
  320 |   await tap(page, "figures", "none");
  321 |   await expect.poll(async () => (await got()).at(-1).figures).toBe("none");
  322 |   await tap(page, "figures", "sprite");
  323 |   await tap(page, "theme", "plain");
  324 |   await expect.poll(async () => (await got()).at(-1)).toMatchObject({ theme: "plain", figures: "ascii", urls: { sprites: null } });
  325 |   expect((await got()).at(-1).vars["--bg"]).toBe("#0d1117");
  326 | });
  327 | 
```