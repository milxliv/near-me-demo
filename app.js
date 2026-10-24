// The location is shown on this screen only. Nothing is sent to a server.
if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('sw.js');
}

const btn = document.getElementById('btn');
const status = document.getElementById('status');

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
      status.textContent = `You are at:\n${latitude.toFixed(5)}, ${longitude.toFixed(5)}\n±${Math.round(accuracy)} m\n\nReady to match nearby people.`;
      btn.disabled = false;
      btn.textContent = 'Update Location';
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
