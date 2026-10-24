// The location is shown on this screen only. It is never sent anywhere.
// After a location, the visitor may type a name; only the name goes to QUOTE_API.
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('sw.js');
}

// Must match connect-src in index.html's Content Security Policy.
const QUOTE_API = 'https://nearme-api.hsca-capecoral.com/api/quote';
const NAME_PATTERN = /^\p{L}[\p{L} .'’-]{0,39}$/u;
const MAX_FIELD = 500;
const LOCATION_ERRORS = {
  1: 'Permission denied. Enable in Settings → Safari → Location.',
  2: 'Location unavailable.',
  3: 'Timed out. Try again.'
};

// Comments, strings, keywords, numbers, called names, then .properties.
const TOKEN = /(\/\/.*)|('(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*"|`(?:[^`\\]|\\.)*`)|\b(async|await|const|function|return|true|false|null)\b|\b(\d+)\b|(\.?[A-Za-z_$][\w$]*)(?=\()|(\.[A-Za-z_$][\w$]*)/g;
const TOKEN_CLASS = [null, 'tok-comment', 'tok-str', 'tok-kw', 'tok-num', 'tok-fn', 'tok-prop'];

const btn = document.getElementById('btn');
const status = document.getElementById('status');
const nameForm = document.getElementById('name-form');
const nameInput = document.getElementById('name');
const nameBtn = document.getElementById('name-btn');
const quoteBox = document.getElementById('quote');
const quoteText = document.getElementById('quote-text');
const quoteCite = document.getElementById('quote-cite');

// The code panel shows the next three functions as their own source text
// (Function.prototype.toString), so it always matches what runs.

function locate(onPosition, onError) {
  // Ask this phone where it is. The answer stays on this phone.
  navigator.geolocation.getCurrentPosition(
    (pos) => onPosition(pos.coords),
    (err) => onError(err),
    { enableHighAccuracy: true, timeout: 12000, maximumAge: 30000 }
  );
}

function showLocation({ latitude, longitude, accuracy }) {
  // Shown here, never sent.
  status.textContent = [
    'You are at:',
    `${latitude.toFixed(5)}, ${longitude.toFixed(5)}`,
    `±${Math.round(accuracy)} m`,
    '',
    "What's up Jack!"
  ].join('\n');
}

async function getQuote(name) {
  // Only the name is sent.
  const res = await fetch(QUOTE_API, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name }),
    credentials: 'omit',
    referrerPolicy: 'no-referrer',
    cache: 'no-store',
    signal: AbortSignal.timeout(8000)
  });
  const data = res.ok ? await res.json() : null;
  return { code: res.status, data };
}

const panel = codePanel(document.getElementById('code'), [locate, showLocation, getQuote]);

btn.onclick = () => {
  if (!navigator.geolocation) {
    status.textContent = 'Location not supported.';
    return;
  }
  btn.disabled = true;
  status.textContent = 'Asking for permission…';
  panel.show([[locate, 'getCurrentPosition(', 'waiting for the location…']], [locate, showLocation]);

  locate(
    (coords) => {
      showLocation(coords);
      const lat = coords.latitude.toFixed(5);
      const lon = coords.longitude.toFixed(5);
      const acc = Math.round(coords.accuracy);
      panel.show([
        [locate, 'onPosition(pos.coords)', `pos.coords: latitude ${lat}, longitude ${lon}, accuracy ${acc} m`],
        [showLocation, '${latitude.toFixed(5)}', `"${lat}, ${lon}"`],
        [showLocation, '${Math.round(accuracy)}', `"±${acc} m"`],
        [showLocation, "What's up Jack!"]
      ], [locate, showLocation]);
      btn.disabled = false;
      btn.textContent = 'Update Location';
      nameForm.hidden = false;
      nameInput.focus();
    },
    (err) => {
      const message = LOCATION_ERRORS[err.code] || 'Error getting location.';
      status.textContent = message;
      panel.show([[locate, 'onError(err)', `err.code ${err.code}: ${message}`]], [locate, showLocation]);
      btn.disabled = false;
    }
  );
};

// Without a citation, the text is a note (status or "no quote found"), not a quote.
function showQuote(text, cite) {
  quoteText.textContent = text;
  quoteCite.textContent = cite;
  quoteCite.hidden = !cite;
  quoteBox.classList.toggle('is-note', !cite);
  quoteBox.hidden = false;
}

nameForm.onsubmit = async (event) => {
  event.preventDefault();
  const name = nameInput.value.trim().replace(/\s+/g, ' ');
  if (!NAME_PATTERN.test(name)) {
    showQuote('Please type a name using letters only, up to 40.', '');
    return;
  }

  nameBtn.disabled = true;
  showQuote('Finding a quote…', '');
  panel.show([
    [getQuote, 'await fetch(QUOTE_API', `POST ${QUOTE_API}`],
    [getQuote, 'JSON.stringify({ name })', JSON.stringify({ name })]
  ], [getQuote]);
  const field = (value) => typeof value === 'string' && value.length > 0 && value.length <= MAX_FIELD;
  try {
    const { code, data } = await getQuote(name);
    if (code === 429) {
      panel.show([[getQuote, 'const data =', 'HTTP 429: too many tries']]);
      showQuote('Too many tries. Wait a minute, then try again.', '');
    } else if (data && field(data.quote) && field(data.citation)) {
      panel.show([[getQuote, 'const data =', JSON.stringify({ quote: data.quote, citation: data.citation })]]);
      showQuote(`“${data.quote}”`, `— ${data.citation}`);
    } else if (data && field(data.message) && !('quote' in data) && !('citation' in data)) {
      // No real quote uses the name: say so, with no quote marks and no citation.
      panel.show([[getQuote, 'const data =', JSON.stringify({ message: data.message })]]);
      showQuote(data.message, '');
    } else {
      throw new Error(`unexpected answer (HTTP ${code})`);
    }
  } catch (err) {
    const reason = err.name === 'TimeoutError' ? 'no answer in 8 s' : err.message;
    panel.show([[getQuote, 'await fetch(QUOTE_API', `failed: ${reason}`]]);
    showQuote("Couldn't get a quote right now. Try again in a moment.", '');
  } finally {
    nameBtn.disabled = false;
  }
};

// Code panel: one element per source line, with syntax colors. show() marks
// the lines that just ran and puts the values they produced under them.
function codePanel(view, functions) {
  const blocks = new Map();
  view.textContent = '';
  functions.forEach((fn, i) => {
    if (i > 0) {
      view.append(codeLine(''));
    }
    const lines = fn.toString().split('\n').map((text) => ({ text, el: codeLine(text) }));
    lines.forEach((line) => view.append(line.el));
    blocks.set(fn, lines);
  });

  return {
    // steps: [fn, text on the line, note?]. fresh: functions whose old notes go.
    show(steps, fresh = []) {
      view.querySelectorAll('.ran').forEach((el) => el.classList.remove('ran'));
      for (const fn of fresh) {
        blocks.get(fn).forEach(({ el }) => el.querySelector('.note')?.remove());
      }
      for (const [fn, marker, note] of steps) {
        const line = blocks.get(fn).find(({ text }) => text.includes(marker));
        if (!line) {
          continue;
        }
        line.el.classList.add('ran');
        line.el.querySelector('.note')?.remove();
        if (note) {
          const span = document.createElement('span');
          span.className = 'note';
          span.textContent = `→ ${note}`;
          line.el.append(span);
        }
      }
    }
  };
}

function codeLine(text) {
  const line = document.createElement('span');
  line.className = 'line';
  let last = 0;
  for (const match of text.matchAll(TOKEN)) {
    let token = match[0];
    line.append(text.slice(last, match.index));
    if (token.startsWith('.')) {
      line.append('.');
      token = token.slice(1);
    }
    const span = document.createElement('span');
    span.className = TOKEN_CLASS[match.findIndex((group, i) => i > 0 && group !== undefined)];
    span.textContent = token;
    line.append(span);
    last = match.index + match[0].length;
  }
  // An empty line still needs height.
  line.append(text.slice(last) || (text ? '' : ' '));
  return line;
}
