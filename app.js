// The location is shown on this screen only. It is never sent anywhere; Copy puts it on
// this phone's clipboard.
// A quote about the nearest place is picked on this phone from places.js.
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
const placeBox = document.getElementById('place');
const placeWhere = document.getElementById('place-where');
const placeText = document.getElementById('place-text');
const placeCite = document.getElementById('place-cite');
const placeNext = document.getElementById('place-next');
const copyBtn = document.getElementById('copy');
// Citations link only to the two sources places.js draws from.
const SOURCE_PAGE = /^https:\/\/(en\.wikiquote\.org\/wiki\/|www\.gutenberg\.org\/ebooks\/)/;
// The place quote on screen, so "Another quote" can step through the same place.
let near = null;
// The coordinates the Copy button copies, and the timer that puts its label back.
let copyText = '';
let copyReset = 0;

// The code panel shows the next five functions as their own source text
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

function quoteNear({ latitude, longitude }) {
  // Picked on this phone from places.js. Nothing is sent.
  const nearby = PLACES.points
    .map(([name, lat, lon, quotes]) => ({ name, quotes, km: distanceKm(latitude, longitude, lat, lon) }))
    .filter((place) => place.km <= PLACES.radiusKm)
    .sort((a, b) => a.km - b.km)[0];
  const [state, country] = PLACES.areas[areaAt(latitude, longitude) - 1] || [-1, -1];
  const region = [state, country]
    .filter((i) => i >= 0)
    .map((i) => ({ name: PLACES.regions[i][0], quotes: PLACES.regions[i][1] }))[0];
  const place = nearby || region;
  if (!place) {
    return { nearby, region };
  }
  const pick = Math.floor(Math.random() * place.quotes.length);
  const [quote, citation, source] = place.quotes[pick];
  return { nearby, region, place, pick, quote, citation, source };
}

function nextQuote(near) {
  // The next quote for the same place, picked on this phone. Nothing is sent.
  const pick = (near.pick + 1) % near.place.quotes.length;
  const [quote, citation, source] = near.place.quotes[pick];
  return { ...near, pick, quote, citation, source };
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

// Great-circle distance in km.
function distanceKm(lat1, lon1, lat2, lon2) {
  const rad = Math.PI / 180;
  const h = Math.sin((lat2 - lat1) * rad / 2) ** 2 +
    Math.cos(lat1 * rad) * Math.cos(lat2 * rad) * Math.sin((lon2 - lon1) * rad / 2) ** 2;
  return 12742 * Math.asin(Math.sqrt(h));
}

// The area number (0 for none) of the grid cell holding a point. Each grid row is
// runs of "area*count", or "area" for a run of one.
function areaAt(latitude, longitude) {
  const { step, rows } = PLACES.grid;
  const row = rows[Math.min(Math.floor((90 - latitude) / step), rows.length - 1)];
  const col = Math.min(Math.floor((longitude + 180) / step), Math.round(360 / step) - 1);
  let end = 0;
  for (const run of row.split(' ')) {
    const [area, count = '1'] = run.split('*');
    end += Number(count);
    if (col < end) {
      return Number(area);
    }
  }
  return 0;
}

const panel = codePanel(document.getElementById('code'),
  [locate, showLocation, quoteNear, nextQuote, getQuote]);

btn.onclick = () => {
  if (!navigator.geolocation) {
    status.textContent = 'Location not supported.';
    return;
  }
  btn.disabled = true;
  showCopy('');
  status.textContent = 'Asking for permission…';
  panel.show([[locate, 'getCurrentPosition(', 'waiting for the location…']],
    [locate, showLocation, quoteNear, nextQuote]);

  locate(
    (coords) => {
      showLocation(coords);
      const lat = coords.latitude.toFixed(5);
      const lon = coords.longitude.toFixed(5);
      const acc = Math.round(coords.accuracy);
      showCopy(`${lat}, ${lon}`);
      const steps = [
        [locate, 'onPosition(pos.coords)', `pos.coords: latitude ${lat}, longitude ${lon}, accuracy ${acc} m`],
        [showLocation, '${latitude.toFixed(5)}', `"${lat}, ${lon}"`],
        [showLocation, '${Math.round(accuracy)}', `"±${acc} m"`],
        [showLocation, "What's up Jack!"]
      ];
      try {
        near = quoteNear(coords);
        steps.push(...showNear(near));
      } catch (err) {
        near = null;
        showPlace('Near you', "Couldn't load the place quotes.");
        steps.push([quoteNear, 'const nearby =', `failed: ${err.message}`]);
      }
      panel.show(steps, [locate, showLocation, quoteNear, nextQuote]);
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

// Shows the Copy button for these coordinates, or hides it for ''. Pages get the
// clipboard only over HTTPS, so without it the button stays hidden.
function showCopy(text) {
  clearTimeout(copyReset);
  copyText = text;
  copyBtn.textContent = 'Copy';
  copyBtn.hidden = !text || !navigator.clipboard;
  status.classList.toggle('has-copy', !copyBtn.hidden);
}

copyBtn.onclick = async () => {
  // Copied to this phone's clipboard only. Nothing is sent.
  clearTimeout(copyReset);
  try {
    await navigator.clipboard.writeText(copyText);
    copyBtn.textContent = 'Copied';
  } catch {
    copyBtn.textContent = "Couldn't copy";
  }
  copyReset = setTimeout(() => { copyBtn.textContent = 'Copy'; }, 1500);
};

// Shows the place quote and returns the panel steps that explain it.
function showNear({ nearby, region, place, quote, citation, source }) {
  const steps = [
    [quoteNear, 'const nearby =', nearby ? `nearby: ${nearby.name}, ${Math.round(nearby.km)} km`
      : `nearby: no quoted place within ${PLACES.radiusKm} km`],
    [quoteNear, 'const [state, country]'],
    [quoteNear, 'const region =', region ? `region: ${region.name}` : 'region: none with a quote']
  ];
  if (!place) {
    showPlace('Near you', 'No famous quote found near you.');
    return [...steps, [quoteNear, 'return { nearby, region };']];
  }
  const away = place === nearby && nearby.km >= 1 ? `, ${Math.round(nearby.km)} km away` : '';
  showPlace(`Near you: ${place.name}${away}`, `“${quote}”`, citation, source, place.quotes.length > 1);
  return [...steps,
    [quoteNear, 'const [quote, citation, source]', JSON.stringify({ quote, citation })]];
}

placeNext.onclick = () => {
  near = nextQuote(near);
  const { quote, citation, source } = near;
  showPlace(placeWhere.textContent, `“${quote}”`, citation, source, true);
  panel.show([[nextQuote, 'const [quote, citation, source]', JSON.stringify({ quote, citation })]],
    [nextQuote]);
};

// Without a citation, the text is a note ("no quote found"), not a quote. The citation
// links to its source page; "Another quote" shows when the place has more than one.
function showPlace(where, text, cite = '', source = '', more = false) {
  placeWhere.textContent = where;
  placeText.textContent = text;
  placeCite.textContent = cite ? '— ' : '';
  if (cite && SOURCE_PAGE.test(source)) {
    const link = document.createElement('a');
    link.href = source;
    link.target = '_blank';
    link.rel = 'noopener noreferrer';
    link.textContent = cite;
    placeCite.append(link);
  } else {
    placeCite.append(cite);
  }
  placeCite.hidden = !cite;
  placeNext.hidden = !more;
  placeBox.classList.toggle('is-note', !cite);
  placeBox.hidden = false;
}

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
