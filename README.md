# Near Me

A small installable web app (PWA) that asks for your location with the browser's standard
permission prompt and shows the coordinates on screen.

Your location stays on your device. The app has no server, no account, no analytics, and no
third-party code. It is plain HTML, CSS, and JavaScript with no build step.

## Try it

1. Open the site on a phone.
2. Tap **Share Location** and answer the permission prompt.
3. To install it:
   - iPhone: Share → Add to Home Screen
   - Android: Menu → Install app

## Files

| File | Purpose |
| --- | --- |
| `index.html` | The page. Its Content Security Policy allows only this site's own files. |
| `app.js` | Asks for the location and shows it. |
| `app.css` | Styles. |
| `sw.js` | Service worker: loads from the network first and falls back to a cache offline. |
| `manifest.json` | Name, icons, and launch settings for the installed app. |
