import { calculateBrightness, normalizeHexValue } from './colors';

// Port of static/js/background-setting.js. Runs after render (replaces the
// DOMContentLoaded wrapper); all behavior kept identical.

let backgroundsArray: string[] = [];

function getEventListeners(element: Element): Record<string, unknown> {
  const el = element as Element & { __events?: Record<string, unknown> };
  return el.__events ?? (el.__events = {});
}

function updateBackgroundsInputValue(backgrounds: string[]): void {
  const modifiedArray = backgrounds.toString().replace(/,/g, "','");
  document
    .getElementById('choose-background-handler')
    ?.setAttribute('value', `['${modifiedArray}']`);
}

function removeBackgroundFromArray(): string[] {
  const container = document.getElementById('choose-backgrounds-container');
  const divs = container?.getElementsByTagName('div') ?? [];
  const next: string[] = [];
  for (const div of Array.from(divs)) {
    const background = div.getAttribute('background');
    if (background) next.push(background);
  }
  console.log(next);
  updateBackgroundsInputValue(next);
  return next;
}

function deleteButtonEvent(event: Event): void {
  backgroundsArray = removeBackgroundFromArray();
  if (backgroundsArray.length !== 1) {
    const target = event.target as Element;
    const divElement = target.closest('.choose-bg-element');
    const backgroundAttribute = divElement?.getAttribute('background') ?? '';
    const filteredBackgrounds = backgroundsArray.filter((item) => !item.startsWith('//'));
    if ((divElement != null && filteredBackgrounds.length !== 1) || backgroundAttribute.startsWith('//')) {
      divElement?.remove();
      backgroundsArray = removeBackgroundFromArray();
    }
  }
}

function activateButtonEvent(event: Event): void {
  backgroundsArray = removeBackgroundFromArray();
  const target = event.target as Element;
  const divElement = target.closest('.choose-bg-element');
  if (divElement != null) {
    const backgroundAttribute = divElement.getAttribute('background') ?? '';
    const activateButton = divElement
      .querySelector('div.choose-bg-buttons')
      ?.querySelector('.choose-bg-activate-button');
    const filteredBackgrounds = backgroundsArray.filter((item) => !item.startsWith('//'));

    if (backgroundAttribute.startsWith('//')) {
      // activate
      divElement.setAttribute('background', backgroundAttribute.replace('//', ''));
      activateButton?.classList.add('choose-bg-activate-button-checked');
      backgroundsArray = removeBackgroundFromArray();
    } else if (filteredBackgrounds.length !== 1) {
      // desactivate
      divElement.setAttribute('background', '//' + backgroundAttribute);
      activateButton?.classList.remove('choose-bg-activate-button-checked');
      backgroundsArray = removeBackgroundFromArray();
    }
  }
}

function wireActivateButtons(): void {
  const activateButtons = document.querySelectorAll('.choose-bg-activate-button');
  activateButtons.forEach((activateButton) => {
    let clickListenerExists = false;
    const clickListeners = getEventListeners(activateButton) as {
      click?: Array<{ listener: unknown }>;
    };
    if (clickListeners && clickListeners.click) {
      for (const entry of clickListeners.click) {
        if (String(entry.listener) === String(activateButtonEvent)) {
          clickListenerExists = true;
          break;
        }
      }
    }
    if (!clickListenerExists) {
      activateButton.addEventListener('click', activateButtonEvent);
    }
  });
}

function wireDeleteButtons(): void {
  const deleteButtons = document.querySelectorAll('.choose-bg-delete-button');
  deleteButtons.forEach((deleteButton) => {
    let clickListenerExists = false;
    const clickListeners = getEventListeners(deleteButton) as {
      click?: Array<{ listener: unknown }>;
    };
    if (clickListeners && clickListeners.click) {
      for (const entry of clickListeners.click) {
        if (String(entry.listener) === String(deleteButtonEvent)) {
          clickListenerExists = true;
          break;
        }
      }
    }
    if (!clickListenerExists) {
      deleteButton.addEventListener('click', deleteButtonEvent);
    }
  });
}

export function initBackgroundSetting(): void {
  function toggleDisplay(): void {
    const chooseBackgroundElement = document.getElementById('choose-background');
    const configContainer = document.getElementById('config-container');
    if (!chooseBackgroundElement || !configContainer) return;

    if (chooseBackgroundElement.style.display === 'none') {
      chooseBackgroundElement.style.display = 'block';
      configContainer.style.display = 'none';
    } else {
      chooseBackgroundElement.style.display = 'none';
      configContainer.style.display = 'block';
    }
  }
  document.getElementById('setting-background')?.addEventListener('click', toggleDisplay);
  document.getElementById('setting-background-back')?.addEventListener('click', toggleDisplay);

  const pageloadElements = document.querySelectorAll('.choose-bg-element-pageload');
  pageloadElements.forEach(function (element) {
    const computedStyle = getComputedStyle(element);
    const backgroundColor = computedStyle.backgroundColor;
    const brightness = calculateBrightness(backgroundColor);
    const htmlElement = element as HTMLElement;
    if (brightness > 125) {
      htmlElement.style.color = 'black';
      element.classList.add('white-text');
    } else {
      htmlElement.style.color = 'white';
    }
  });

  const buttonBackgroundColorInput = document.getElementById(
    'background-color-input'
  ) as HTMLInputElement | null;
  const buttonBackgroundColorHex = document.getElementById(
    'background-color-hex'
  ) as HTMLInputElement | null;

  buttonBackgroundColorInput?.addEventListener('input', function () {
    if (!buttonBackgroundColorInput || !buttonBackgroundColorHex) return;
    const hexValue = normalizeHexValue(buttonBackgroundColorInput.value);
    buttonBackgroundColorInput.value = hexValue;
    buttonBackgroundColorHex.value = hexValue;
  });

  buttonBackgroundColorHex?.addEventListener('input', function () {
    if (!buttonBackgroundColorInput || !buttonBackgroundColorHex) return;
    const normalizedHexValue = normalizeHexValue(buttonBackgroundColorHex.value);
    buttonBackgroundColorInput.value = normalizedHexValue;
    buttonBackgroundColorHex.value = normalizedHexValue;
  });

  const chooseBackground = document.getElementById('choose-background-handler') as HTMLInputElement | null;
  if (chooseBackground) {
    const backgroundsArrayString = chooseBackground.value
      .replace("['", '["')
      .replace("']", '"]')
      .replace("','", '","');
    backgroundsArray = JSON.parse(backgroundsArrayString) as string[];
  }

  document.getElementById('create-color-bg')?.addEventListener('click', function () {
    const colorHex = (document.getElementById('background-color-hex') as HTMLInputElement | null)?.value ?? '';
    if (colorHex !== '') {
      const divElement = document.createElement('div');
      divElement.classList.add('choose-bg-element');
      divElement.classList.add('choose-bg-element-color');
      divElement.style.backgroundColor = colorHex;
      const createColorBgElement = document.getElementById('create-color-bg');
      if (createColorBgElement?.classList.contains('dark-theme')) {
        divElement.classList.add('dark-theme');
      }

      const brightness = calculateBrightness(colorHex);
      if (brightness > 125) {
        divElement.style.color = '#141414';
      } else {
        divElement.style.color = '#fbfbfd';
      }

      backgroundsArray.push(colorHex);

      const container = document.getElementById('choose-backgrounds-container');
      const divs = container?.getElementsByTagName('div');
      const background_color_text = divs?.[0]?.getAttribute('background_color_text') ?? '';

      divElement.textContent = background_color_text + ' : ' + colorHex;
      divElement.setAttribute('background', colorHex);

      document.getElementById('choose-backgrounds-container')?.appendChild(divElement);

      updateBackgroundsInputValue(backgroundsArray);

      const chooseBgButtonsDiv = document.querySelector('.choose-bg-buttons');
      const clonedChooseBgButtons = chooseBgButtonsDiv?.cloneNode(true) as Element | undefined;
      if (clonedChooseBgButtons) {
        divElement.appendChild(clonedChooseBgButtons);
        const activateButton = clonedChooseBgButtons.querySelector('.choose-bg-activate-button');
        activateButton?.classList.add('choose-bg-activate-button-checked');
      }

      wireActivateButtons();
      wireDeleteButtons();

      console.log(backgroundsArray.length + backgroundsArray.join(','));
    }
  });

  document.getElementById('create-image-bg')?.addEventListener('change', function () {
    const input = this as unknown as HTMLInputElement;

    if (input.files && input.files[0]) {
      const file = input.files[0];
      const formData = new FormData();
      formData.append('file', file);
      formData.append('info', 'background_image');

      const xhr = new XMLHttpRequest();
      xhr.open('POST', '/upload_file', true);
      xhr.onload = function () {
        if (xhr.status === 200) {
          console.log('File downloaded successfully');
          const firstFile = input.files?.[0];
          const fileName = '**uploaded/' + (firstFile?.name ?? '');
          console.log('Image sent successfully! File name :', fileName);

          const imageFile = '.config/user_uploads/' + (firstFile?.name ?? '');

          const divElement = document.createElement('div');
          divElement.classList.add('choose-bg-element');
          divElement.classList.add('choose-bg-element-image');
          const createColorBgElement = document.getElementById('create-color-bg');
          if (createColorBgElement?.classList.contains('dark-theme')) {
            divElement.classList.add('dark-theme');
          }

          if (!imageFile.endsWith('.mp4')) {
            const pseudoElement = document.createElement('div');
            pseudoElement.classList.add('choose-bg-pseudo-element');
            pseudoElement.style.backgroundImage = 'url("' + imageFile + '")';
            divElement.appendChild(pseudoElement);
          }

          backgroundsArray.push(fileName);
          divElement.setAttribute('background', fileName);

          document.getElementById('choose-backgrounds-container')?.appendChild(divElement);

          let mediaElement: HTMLElement | undefined;

          if (imageFile.endsWith('.mp4')) {
            const videoContainerBlurred = document.createElement('div');
            videoContainerBlurred.className = 'video-container choose-bg-pseudo-element';
            const videoElementBlurred = document.createElement('video');
            videoElementBlurred.autoplay = true;
            videoElementBlurred.muted = true;
            videoElementBlurred.loop = true;
            videoElementBlurred.className = 'blurred-video';
            const sourceElementBlurred = document.createElement('source');
            sourceElementBlurred.src = imageFile;
            sourceElementBlurred.type = 'video/mp4';
            videoElementBlurred.appendChild(sourceElementBlurred);
            videoContainerBlurred.appendChild(videoElementBlurred);

            const videoContainer = document.createElement('div');
            videoContainer.className = 'video-container';
            const videoElement = document.createElement('video');
            videoElement.autoplay = true;
            videoElement.muted = true;
            videoElement.loop = true;
            const sourceElement = document.createElement('source');
            sourceElement.src = imageFile;
            sourceElement.type = 'video/mp4';
            videoElement.appendChild(sourceElement);
            videoContainer.appendChild(videoElement);

            divElement.appendChild(videoContainerBlurred);
            divElement.appendChild(videoContainer);
            console.log(mediaElement);
          } else {
            const imgElement = document.createElement('img');
            imgElement.setAttribute('src', imageFile);
            mediaElement = imgElement;
          }
          void mediaElement;

          const chooseBgButtonsDiv = document.querySelector('.choose-bg-buttons');
          const clonedChooseBgButtons = chooseBgButtonsDiv?.cloneNode(true) as Element | undefined;
          if (clonedChooseBgButtons) {
            divElement.appendChild(clonedChooseBgButtons);
            const activateButton = clonedChooseBgButtons.querySelector('.choose-bg-activate-button');
            activateButton?.classList.add('choose-bg-activate-button-checked');
          }

          wireActivateButtons();
          wireDeleteButtons();
          removeBackgroundFromArray();
        } else {
          console.error('Failed to download file.');
        }
      };
      xhr.send(formData);
    }
  });

  wireDeleteButtons();
  wireActivateButtons();
}

console.log('background-setting.js loaded');
