# Near Me

A small installable web app (PWA) that asks for your location with the browser's standard
permission prompt, shows the coordinates on screen, and then offers a famous quote that uses
your name.

Your location stays on your device. It is shown on the screen and never recorded or sent
anywhere. If you type your name, only the name goes to a small quote service, which keeps
nothing and logs no requests. There is no account, no analytics, and no third-party code. The
page is plain HTML, CSS, and JavaScript with no build step.

## Try it

1. Open the site on a phone.
2. Tap **Share Location** and answer the permission prompt.
3. Type your name and tap **Get my quote**.
4. To install it:
   - iPhone: Share → Add to Home Screen
   - Android: Menu → Install app

## Files

| File | Purpose |
| --- | --- |
| `index.html` | The page. Its Content Security Policy allows only this site's own files, plus the one quote endpoint. |
| `app.js` | Asks for the location and shows it; sends only the name to the quote service. |
| `app.css` | Styles. |
| `sw.js` | Service worker: loads from the network first and falls back to a cache offline. |
| `manifest.json` | Name, icons, and launch settings for the installed app. |
| `quote-api/` | The quote service (Rust). |

## Quote service

`quote-api/` is a single Rust binary with one endpoint:

```
POST /api/quote   {"name": "Jack"}   ->   {"quote": "...", "citation": "..."}
```

- Quotes come from a fixed list that was checked by hand, such as lines from films, plays,
  songs, and memes that use a first name. No search engine or AI writes a quote or a citation
  at request time.
- A name with no match gets a famous line with the name put in. Its citation starts with
  "Adapted from".
- It accepts only `{"name": "..."}` as JSON from this site's origin. A name is 1 to 40 letters,
  spaces, hyphens, apostrophes, or periods.
- Each visitor gets 10 requests a minute, and the service takes 300 a minute in total.
- It stores nothing and logs no requests. It listens on localhost behind Cloudflare Tunnel, and
  its systemd sandbox forbids outside connections (`near-me-quote.service`).

Run the tests with `cargo test` in `quote-api/`.
