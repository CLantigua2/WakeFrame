const videos = [
  { name: 'Mountain Lake', duration: '4s', selected: true },
  { name: 'Forest Path', duration: '6s', selected: true },
  { name: 'Ocean Sunset', duration: '5s', selected: true },
  { name: 'City Night', duration: '7s', selected: false },
  { name: 'Snowy Cabin', duration: '6s', selected: false }
];

const timeBlock = document.getElementById('timeBlock');
const dateBlock = document.getElementById('dateBlock');
const videoList = document.getElementById('videoList');
const selectionSummary = document.getElementById('selectionSummary');
const previewBtn = document.getElementById('previewBtn');
const previewPanel = document.querySelector('.preview-panel');
const mainToggle = document.getElementById('mainToggle');
const trayToggle = document.getElementById('trayToggle');
const disabledToggle = document.getElementById('disabledToggle');
const randomToggle = document.getElementById('randomToggle');
const settingsBtn = document.getElementById('settingsBtn');
const addVideoBtn = document.getElementById('addVideoBtn');

function updateClock() {
  const now = new Date();
  const time = now.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
  const date = now.toLocaleDateString([], { month: 'numeric', day: 'numeric', year: 'numeric' });
  timeBlock.textContent = time;
  dateBlock.textContent = date;
}

function updateSummary() {
  const selectedCount = videos.filter((video) => video.selected).length;
  selectionSummary.textContent = `${videos.length} videos • ${selectedCount} selected`;
}

function renderVideos() {
  videoList.innerHTML = videos
    .map(
      (video, index) => `
        <div class="video-item ${video.selected ? 'selected' : ''}" data-index="${index}" tabindex="0" role="button" aria-label="Toggle ${video.name}">
          <span class="checkmark" aria-hidden="true"></span>
          <div class="video-item-main">
            <span class="video-thumb" aria-hidden="true"></span>
            <span class="video-name">${video.name}</span>
          </div>
          <span class="video-duration">${video.duration}</span>
          <button class="eye-btn" type="button" aria-label="Preview ${video.name}"></button>
        </div>
      `
    )
    .join('');

  updateSummary();

  document.querySelectorAll('.video-item').forEach((item) => {
    item.addEventListener('click', (event) => {
      const index = Number(item.dataset.index);
      if (event.target.closest('.eye-btn')) {
        previewBtn.click();
        return;
      }
      videos[index].selected = !videos[index].selected;
      renderVideos();
    });

    item.addEventListener('keydown', (event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        const index = Number(item.dataset.index);
        videos[index].selected = !videos[index].selected;
        renderVideos();
      }
    });
  });
}

function setSwitch(el, enabled) {
  el.classList.toggle('on', enabled);
  el.setAttribute('aria-pressed', String(enabled));
}

mainToggle.addEventListener('click', () => {
  const enabled = !mainToggle.classList.contains('on');
  setSwitch(mainToggle, enabled);
  setSwitch(trayToggle, enabled);
  setSwitch(disabledToggle, enabled);
});

trayToggle.addEventListener('click', () => {
  const enabled = !trayToggle.classList.contains('on');
  setSwitch(trayToggle, enabled);
  setSwitch(mainToggle, enabled);
  setSwitch(disabledToggle, enabled);
});

disabledToggle.addEventListener('click', () => {
  const enabled = !disabledToggle.classList.contains('on');
  setSwitch(disabledToggle, enabled);
  setSwitch(mainToggle, enabled);
  setSwitch(trayToggle, enabled);
});

randomToggle.addEventListener('click', () => {
  const enabled = !randomToggle.classList.contains('on');
  setSwitch(randomToggle, enabled);
});

previewBtn.addEventListener('click', () => {
  previewPanel.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
});

settingsBtn.addEventListener('click', () => {
  document.querySelector('.settings-card').scrollIntoView({ behavior: 'smooth', block: 'nearest' });
});

addVideoBtn.addEventListener('click', () => {
  document.querySelector('.add-card').scrollIntoView({ behavior: 'smooth', block: 'nearest' });
});

updateClock();
renderVideos();
setSwitch(mainToggle, true);
setSwitch(trayToggle, true);
setSwitch(disabledToggle, true);
setSwitch(randomToggle, true);
