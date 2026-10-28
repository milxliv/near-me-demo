// Draws places.html from places.js (the file the app picks from) and land.js.
// Nothing on this page asks for the location or sends anything.

const map = document.getElementById('map');
const readout = document.getElementById('readout');
const find = document.getElementById('find');
const count = document.getElementById('count');
const list = document.getElementById('list');
const HINT = readout.textContent;
// Citations link only to the two sources places.js draws from.
const SOURCE_PAGE = /^https:\/\/(en\.wikiquote\.org\/wiki\/|www\.gutenberg\.org\/ebooks\/)/;
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');

// The map shows 84° N to 60° S; no quoted place lies nearer the poles.
const TOP = 84;
const BOTTOM = -60;
const STEP = PLACES.grid.step;
const ROW0 = Math.round((90 - TOP) / STEP);
const ROWS = Math.round((TOP - BOTTOM) / STEP);
const COLS = Math.round(360 / STEP);
const LAND_COLS = Math.round(360 / LAND.step);

// Each grid row is runs of "value*count", or "value" for a run of one.
function decode(row, out, at) {
  for (const run of row.split(' ')) {
    const [value, n = '1'] = run.split('*');
    out.fill(Number(value), at, at + Number(n));
    at += Number(n);
  }
}

// Area number (0 for none) of every shown cell, and whether its half-degree cell is land.
const cover = new Uint16Array(ROWS * COLS);
const land = new Uint8Array(LAND.rows.length * LAND_COLS);
for (let r = 0; r < ROWS; r++) decode(PLACES.grid.rows[ROW0 + r], cover, r * COLS);
LAND.rows.forEach((row, r) => decode(row, land, r * LAND_COLS));

// A region is listed as a country if any cell uses it as one; the rest are states and regions.
const countries = new Set(PLACES.areas.map(([, c]) => c).filter((i) => i >= 0));
const states = new Set(PLACES.areas.map(([s]) => s).filter((i) => i >= 0 && !countries.has(i)));

const byName = (a, b) => a.name.localeCompare(b.name);
const groups = [
  ['Towns and landmarks', 'within 30 km',
    PLACES.points.map(([name, , , quotes], i) => ({ id: `p${i}`, name, quotes }))],
  ['States and regions', 'state or region',
    [...states].map((i) => ({ id: `r${i}`, name: PLACES.regions[i][0], quotes: PLACES.regions[i][1] }))],
  ['Countries', 'country',
    [...countries].map((i) => ({ id: `r${i}`, name: PLACES.regions[i][0], quotes: PLACES.regions[i][1] }))]
].map(([title, kind, items]) => ({ title, kind, items: items.sort(byName) }));
const TOTAL = groups.reduce((n, group) => n + group.items.length, 0);

function el(tag, props = {}, ...children) {
  const node = Object.assign(document.createElement(tag), props);
  node.append(...children);
  return node;
}

function placeCard(place) {
  const card = el('article', { className: 'place', id: place.id },
    el('h3', {}, place.name, ' ', el('small', {}, `${place.quotes.length} ${place.quotes.length === 1 ? 'quote' : 'quotes'}`)));
  for (const [text, citation, source] of place.quotes) {
    const cite = el('figcaption', {}, '— ');
    if (SOURCE_PAGE.test(source)) {
      cite.append(el('a', { href: source, target: '_blank', rel: 'noopener noreferrer', textContent: citation }));
    } else {
      cite.append(citation);
    }
    card.append(el('figure', {}, el('blockquote', { textContent: `“${text}”` }), cite));
  }
  // Text the finder matches: the name, the quotes, and who said them.
  place.search = [place.name, ...place.quotes.flat()].join(' ').toLowerCase();
  place.card = card;
  return card;
}

for (const group of groups) {
  group.count = el('span');
  group.section = el('section', { className: 'group' },
    el('h2', {}, group.title, ' ', group.count),
    el('div', { className: 'place-grid' }, ...group.items.map(placeCard)));
  list.append(group.section);
}

function filter() {
  const words = find.value.toLowerCase().split(/\s+/).filter(Boolean);
  let shown = 0;
  for (const group of groups) {
    let inGroup = 0;
    for (const place of group.items) {
      const hit = words.every((w) => place.search.includes(w));
      place.card.hidden = !hit;
      inGroup += hit;
    }
    group.count.textContent = inGroup;
    group.section.hidden = inGroup === 0;
    shown += inGroup;
  }
  count.textContent = !shown ? 'No place matches. Try a country or a state.'
    : words.length ? `${shown} of ${TOTAL} places` : `${TOTAL} places`;
}
find.addEventListener('input', filter);
filter();

// Map drawing. Colors come from the page's CSS, so the map follows light and dark mode.
function hex(name) {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return [1, 3, 5].map((i) => parseInt(v.slice(i, i + 2), 16));
}

function toCanvas(lat, lon) {
  return [(lon + 180) / STEP, (TOP - lat) / STEP];
}

function draw() {
  const ctx = map.getContext('2d');
  const img = ctx.createImageData(COLS, ROWS);
  const [sea, ground, country, state] =
    ['--map-sea', '--map-land', '--map-country', '--map-state'].map(hex);
  const half = LAND.step / STEP;
  for (let r = 0; r < ROWS; r++) {
    const landRow = Math.floor((ROW0 + r) / half) * LAND_COLS;
    for (let c = 0; c < COLS; c++) {
      const cell = cover[r * COLS + c];
      const area = cell ? PLACES.areas[cell - 1] : null;
      const rgb = area ? (states.has(area[0]) ? state : country)
        : land[landRow + Math.floor(c / half)] ? ground : sea;
      const o = (r * COLS + c) * 4;
      img.data[o] = rgb[0];
      img.data[o + 1] = rgb[1];
      img.data[o + 2] = rgb[2];
      img.data[o + 3] = 255;
    }
  }
  ctx.putImageData(img, 0, 0);
  // Dots keep about the same size on screen however wide the map is drawn.
  const scale = map.width / (map.getBoundingClientRect().width || map.width);
  const [dot, ring] = ['--map-dot', '--map-dot-ring'].map(hex);
  ctx.fillStyle = `rgb(${dot})`;
  ctx.strokeStyle = `rgb(${ring})`;
  ctx.lineWidth = 1.2 * scale;
  for (const [, lat, lon] of PLACES.points) {
    const [x, y] = toCanvas(lat, lon);
    ctx.beginPath();
    ctx.arc(x, y, 3.5 * scale, 0, 2 * Math.PI);
    ctx.fill();
    ctx.stroke();
  }
}

// The place under a point on the map: a dot within 14 screen pixels, else the shaded area.
function placeAt(event) {
  const box = map.getBoundingClientRect();
  const scale = map.width / box.width;
  const x = (event.clientX - box.left) * scale;
  const y = (event.clientY - box.top) * scale;
  let best = null;
  let bestDist = (14 * scale) ** 2;
  PLACES.points.forEach(([name, lat, lon], i) => {
    const [px, py] = toCanvas(lat, lon);
    const dist = (px - x) ** 2 + (py - y) ** 2;
    if (dist < bestDist) {
      bestDist = dist;
      best = { id: `p${i}`, label: `${name} · town or landmark` };
    }
  });
  if (best) return best;
  const row = Math.floor(y);
  const col = Math.floor(x);
  if (row < 0 || row >= ROWS || col < 0 || col >= COLS) return null;
  const cell = cover[row * COLS + col];
  if (!cell) return null;
  const [s, c] = PLACES.areas[cell - 1];
  const i = s >= 0 ? s : c;
  return { id: `r${i}`, label: `${PLACES.regions[i][0]} · ${countries.has(i) ? 'country' : 'state or region'}` };
}

map.addEventListener('click', (event) => {
  const hit = placeAt(event);
  if (!hit) {
    readout.textContent = 'No quote for that spot yet.';
    return;
  }
  readout.textContent = hit.label;
  const card = document.getElementById(hit.id);
  if (card.hidden) {
    find.value = '';
    filter();
  }
  card.scrollIntoView({ behavior: reducedMotion.matches ? 'auto' : 'smooth', block: 'center' });
  card.classList.add('flash');
  setTimeout(() => card.classList.remove('flash'), 1800);
});
map.addEventListener('pointermove', (event) => {
  if (event.pointerType !== 'mouse') return;
  const hit = placeAt(event);
  readout.textContent = hit ? hit.label : HINT;
});
map.addEventListener('pointerleave', () => { readout.textContent = HINT; });

matchMedia('(prefers-color-scheme: dark)').addEventListener('change', draw);
new ResizeObserver(draw).observe(map);
draw();
