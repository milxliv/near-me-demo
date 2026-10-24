// The location is shown on this screen only. It is never sent anywhere.
// After a location, the visitor may type a name; only the name goes to QUOTE_API.
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('sw.js');
}

// Must match connect-src in index.html's Content Security Policy.
const QUOTE_API = 'https://nearme-api.hsca-capecoral.com/api/quote';
const NAME_PATTERN = /^\p{L}[\p{L} .'’-]{0,39}$/u;
const MAX_FIELD = 500;

const btn = document.getElementById('btn');
const status = document.getElementById('status');
const nameForm = document.getElementById('name-form');
const nameInput = document.getElementById('name');
const nameBtn = document.getElementById('name-btn');
const quoteBox = document.getElementById('quote');
const quoteText = document.getElementById('quote-text');
const quoteCite = document.getElementById('quote-cite');

btn.onclick = () => {
  if (!navigator.geolocation) {
    status.textContent = 'Location not supported.';
    return;
  }
  btn.disabled = true;
  status.textContent = 'Asking for permission…';

  navigator.geolocation.getCurrentPosition(
    (pos) => {
      const { latitude, longitude, accuracy } = pos.coords;
      status.textContent = `You are at:\n${latitude.toFixed(5)}, ${longitude.toFixed(5)}\n±${Math.round(accuracy)} m\n\nWhat's up Jack!`;
      btn.disabled = false;
      btn.textContent = 'Update Location';
      nameForm.hidden = false;
      nameInput.focus();
    },
    (err) => {
      const messages = {
        1: 'Permission denied. Enable in Settings → Safari → Location.',
        2: 'Location unavailable.',
        3: 'Timed out. Try again.'
      };
      status.textContent = messages[err.code] || 'Error getting location.';
      btn.disabled = false;
    },
    { enableHighAccuracy: true, timeout: 12000, maximumAge: 30000 }
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
  const timeout = new AbortController();
  const timer = setTimeout(() => timeout.abort(), 8000);
  try {
    const res = await fetch(QUOTE_API, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name }),
      credentials: 'omit',
      referrerPolicy: 'no-referrer',
      cache: 'no-store',
      signal: timeout.signal
    });
    if (res.status === 429) {
      showQuote('Too many tries. Wait a minute, then try again.', '');
      return;
    }
    const data = res.ok ? await res.json() : null;
    const field = (value) => typeof value === 'string' && value.length > 0 && value.length <= MAX_FIELD;
    if (data && field(data.quote) && field(data.citation)) {
      showQuote(`“${data.quote}”`, `— ${data.citation}`);
    } else if (data && field(data.message) && !('quote' in data) && !('citation' in data)) {
      // No real quote uses the name: say so, with no quote marks and no citation.
      showQuote(data.message, '');
    } else {
      throw new Error('bad response');
    }
  } catch {
    showQuote("Couldn't get a quote right now. Try again in a moment.", '');
  } finally {
    clearTimeout(timer);
    nameBtn.disabled = false;
  }
};
