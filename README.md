# Near Me

A small installable web app (PWA) that asks for your location with the browser's standard
permission prompt, shows the coordinates on screen, and then finds a famous quote that uses
your name.

Your location stays on your device. It is shown on the screen and never recorded or sent
anywhere. If you type your name, only the name goes to a small quote service, which keeps
nothing and logs no requests. The service looks the name up on Wikiquote. There is no account,
no analytics, and no third-party code in the page. The page is plain HTML, CSS, and JavaScript
with no build step.

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
| `app.js` | Asks for the location and shows it; sends only the name to the quote service. The code panel on the page shows this file's functions as they run, with the values they produce. |
| `app.css` | Styles. |
| `sw.js` | Service worker: loads from the network first and falls back to a cache offline. |
| `manifest.json` | Name, icons, and launch settings for the installed app. |
| `quote-api/` | The quote service (Rust). |

## Quote service

`quote-api/` is a single Rust binary with one endpoint:

```
POST /api/quote   {"name": "Jack"}   ->   {"quote": "...", "citation": "..."}
POST /api/quote   {"name": "Qwzx"}   ->   {"message": "No famous quote found for Qwzx."}
```

- The service searches [Wikiquote](https://en.wikiquote.org) for a line that uses the name and
  returns it with its speaker and work, ending in "via Wikiquote". Film and TV lines come first.
  No AI writes a quote or a citation.
- Only the first word of the name goes from the service to Wikiquote, as a search. Wikiquote
  sees the server's address, not yours, and none of your browser's headers.
- A word filter drops lines with profanity, slurs, or graphic violence. It is a filter, not a
  guarantee.
- If Wikiquote finds nothing usable, fails, or takes more than 4 seconds, a short list checked
  by hand answers instead. Every quote on it contains its name.
- If neither has a quote that uses the name, the answer is a message with no quote and no
  citation. The service never puts a name into a quote that does not already contain it.
- It accepts only `{"name": "..."}` as JSON from this site's origin. A name is 1 to 40 letters,
  spaces, hyphens, apostrophes, or periods.
- Each visitor gets 10 requests a minute, and the service takes 300 a minute in total.
- It makes at most 30 Wikiquote lookups a minute in total; past that, the fixed list answers.
- It stores nothing and logs no requests. It listens on localhost behind Cloudflare Tunnel, and
  its systemd sandbox allows outside connections only to Wikimedia's addresses
  (`near-me-quote.service`). Setting `NEARME_LIVE_LOOKUP=off` turns the lookup off.

Run the tests with `cargo test` in `quote-api/`.
