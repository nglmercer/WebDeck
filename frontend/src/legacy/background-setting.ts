import { uploadFile } from '../api/uploads';
import { q, byId } from '../query';
import { calculateBrightness, normalizeHexValue } from './colors';

// Port of static/js/background-setting.js. Runs after render (replaces the
// DOMContentLoaded wrapper); all behavior kept identical.

let backgroundsArray: string[] = [];

function getEventListeners(element: Element): Record<string, unknown> {
  // Custom expando protocol (upstream devtools-style check); qdom's
  // registry-keyed .data() is different storage, so this stays native.
  const el = element as Element & { __events?: Record<string, unknown> };
  return el.__events ?? (el.__events = {});
}

function updateBackgroundsInputValue(backgrounds: string[]): void {
  const modifiedArray = backgrounds.toString().replace(/,/g, "','");
  // NOTE: the *attribute* (default value), not the property — .attr(), not .val().
  byId('choose-background-handler').attr('value', `['${modifiedArray}']`);
}

function removeBackgroundFromArray(): string[] {
  const container = byId('choose-backgrounds-container').get(0) ?? null;
  const next: string[] = [];
  // NOTE: scoped via .find(), not the context parameter (a null context
  // would fall back to the whole document; a missing container yields no
  // divs, like the original `?.` chain).
  for (const div of q(container).find('div').toArray()) {
    const background = q(div).attr('background');
    if (background) next.push(background);
  }
  updateBackgroundsInputValue(next);
  return next;
}

function refreshBgCount(): void {
  const container = byId('choose-backgrounds-container').get(0) ?? null;
  const count = container
    ? q(container)
        .children()
        .toArray()
        .filter((el) => q(el).hasClass('choose-bg-element')).length
    : 0;
  const badge = byId('bg-count').get(0) ?? null;
  if (badge) badge.textContent = String(count);
}

function deleteButtonEvent(event: Event): void {
  backgroundsArray = removeBackgroundFromArray();
  if (backgroundsArray.length !== 1) {
    const target = event.target as Element;
    const divElement = q(target).closest('.choose-bg-element').get(0) ?? null;
    const backgroundAttribute = q(divElement).attr('background') ?? '';
    const filteredBackgrounds = backgroundsArray.filter((item) => !item.startsWith('//'));
    if ((divElement != null && filteredBackgrounds.length !== 1) || backgroundAttribute.startsWith('//')) {
      q(divElement).remove();
      backgroundsArray = removeBackgroundFromArray();
      refreshBgCount();
    }
  }
}

function activateButtonEvent(event: Event): void {
  backgroundsArray = removeBackgroundFromArray();
  const target = event.target as Element;
  const divElement = q(target).closest('.choose-bg-element').get(0) ?? null;
  if (divElement != null) {
    const backgroundAttribute = q(divElement).attr('background') ?? '';
    const activateButton = q(divElement).find('div.choose-bg-buttons .choose-bg-activate-button').get(0) ?? null;
    const filteredBackgrounds = backgroundsArray.filter((item) => !item.startsWith('//'));

    if (backgroundAttribute.startsWith('//')) {
      // activate
      q(divElement).attr('background', backgroundAttribute.replace('//', ''));
      q(activateButton).addClass('choose-bg-activate-button-checked');
      q(activateButton).attr('aria-pressed', 'true');
      backgroundsArray = removeBackgroundFromArray();
    } else if (filteredBackgrounds.length !== 1) {
      // desactivate
      q(divElement).attr('background', '//' + backgroundAttribute);
      q(activateButton).removeClass('choose-bg-activate-button-checked');
      q(activateButton).attr('aria-pressed', 'false');
      backgroundsArray = removeBackgroundFromArray();
    }
  }
}

function wireActivateButtons(): void {
  q('.choose-bg-activate-button')
    .toArray()
    .forEach((activateButton) => {
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
        q(activateButton).on('click', activateButtonEvent);
      }
    });
}

function wireDeleteButtons(): void {
  q('.choose-bg-delete-button')
    .toArray()
    .forEach((deleteButton) => {
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
        q(deleteButton).on('click', deleteButtonEvent);
      }
    });
}

export function initBackgroundSetting(): void {
  // NOTE: panel visibility is owned by the config tab state — no
  // back-button navigation. Only picker wiring below.
  q('.choose-bg-element-pageload')
    .toArray()
    .forEach(function (element) {
      const backgroundColor = q(element).css('backgroundColor') ?? '';
      const brightness = calculateBrightness(backgroundColor);
      if (brightness > 125) {
        q(element).css('color', 'black');
        q(element).addClass('white-text');
      } else {
        q(element).css('color', 'white');
      }
    });

  const buttonBackgroundColorInput = byId<HTMLInputElement>('background-color-input').get(0) ?? null;
  const buttonBackgroundColorHex = byId<HTMLInputElement>('background-color-hex').get(0) ?? null;

  q(buttonBackgroundColorInput).on('input', function () {
    if (!buttonBackgroundColorInput || !buttonBackgroundColorHex) return;
    const hexValue = normalizeHexValue(String(q(buttonBackgroundColorInput).val() ?? ''));
    q(buttonBackgroundColorInput).val(hexValue);
    q(buttonBackgroundColorHex).val(hexValue);
  });

  q(buttonBackgroundColorHex).on('input', function () {
    if (!buttonBackgroundColorInput || !buttonBackgroundColorHex) return;
    const normalizedHexValue = normalizeHexValue(String(q(buttonBackgroundColorHex).val() ?? ''));
    q(buttonBackgroundColorInput).val(normalizedHexValue);
    q(buttonBackgroundColorHex).val(normalizedHexValue);
  });

  const chooseBackground = byId<HTMLInputElement>('choose-background-handler').get(0) ?? null;
  if (chooseBackground) {
    const backgroundsArrayString = String(q(chooseBackground).val() ?? '')
      .replace("['", '["')
      .replace("']", '"]')
      .replace("','", '","');
    backgroundsArray = JSON.parse(backgroundsArrayString) as string[];
  }

  byId('create-color-bg').on('click', function () {
    const colorHex = String(byId<HTMLInputElement>('background-color-hex').val() ?? '');
    if (colorHex !== '') {
      const divElement = q('<div>')
        .addClass('choose-bg-element')
        .addClass('choose-bg-element-color')
        .css('backgroundColor', colorHex);
      if (byId('create-color-bg').hasClass('dark-theme')) {
        divElement.addClass('dark-theme');
      }

      const brightness = calculateBrightness(colorHex);
      if (brightness > 125) {
        divElement.css('color', '#141414');
      } else {
        divElement.css('color', '#fbfbfd');
      }

      backgroundsArray.push(colorHex);

      const container = byId('choose-backgrounds-container').get(0) ?? null;
      const background_color_text = container
        ? q('div', container).attr('background_color_text') ?? ''
        : '';

      divElement.attr('background', colorHex);
      divElement.append(q('<span>').addClass('choose-bg-swatch').css('backgroundColor', colorHex));
      const foot = q('<div>').addClass('choose-bg-foot');
      foot.append(
        q('<span>').addClass('choose-bg-label').text(background_color_text + ' · ' + colorHex)
      );

      byId('choose-backgrounds-container').append(divElement);

      updateBackgroundsInputValue(backgroundsArray);

      const clonedChooseBgButtons = q('.choose-bg-buttons').clone().get(0);
      if (clonedChooseBgButtons) {
        foot.append(clonedChooseBgButtons);
        q(clonedChooseBgButtons)
          .find('.choose-bg-activate-button')
          .addClass('choose-bg-activate-button-checked')
          .attr('aria-pressed', 'true');
      }
      divElement.append(foot);

      wireActivateButtons();
      wireDeleteButtons();
      refreshBgCount();
    }
  });

  byId('create-image-bg').on('change', function () {
    const input = this as unknown as HTMLInputElement;

    if (input.files && input.files[0]) {
      const file = input.files[0];
      const formData = new FormData();
      formData.append('file', file);
      formData.append('info', 'background_image');

      void uploadFile(formData).then(
        () => {
          const firstFile = input.files?.[0];
          const fileName = '**uploaded/' + (firstFile?.name ?? '');

          const imageFile = '.config/user_uploads/' + (firstFile?.name ?? '');

          const divElement = q('<div>').addClass('choose-bg-element').addClass('choose-bg-element-image');
          if (byId('create-color-bg').hasClass('dark-theme')) {
            divElement.addClass('dark-theme');
          }

          const thumb = q('<div>').addClass('choose-bg-thumb');
          if (!imageFile.endsWith('.mp4')) {
            thumb.append(
              q('<div>').addClass('choose-bg-pseudo-element').css('backgroundImage', 'url("' + imageFile + '")')
            );
            // Sharp image on top of the blurred fill (matches SSR cards).
            thumb.append(q<HTMLImageElement>('<img>').attr('src', imageFile).attr('alt', ''));
          }

          backgroundsArray.push(fileName);
          divElement.attr('background', fileName);

          byId('choose-backgrounds-container').append(divElement);

          if (imageFile.endsWith('.mp4')) {
            const videoElementBlurred = q<HTMLVideoElement>('<video>')
              .prop({ autoplay: true, muted: true, loop: true })
              .attr('class', 'blurred-video');
            videoElementBlurred.append(
              q<HTMLSourceElement>('<source>').prop('src', imageFile).prop('type', 'video/mp4')
            );
            const videoContainerBlurred = q('<div>').attr(
              'class',
              'video-container choose-bg-pseudo-element'
            );
            videoContainerBlurred.append(videoElementBlurred);

            const videoElement = q<HTMLVideoElement>('<video>').prop({
              autoplay: true,
              muted: true,
              loop: true,
            });
            videoElement.append(
              q<HTMLSourceElement>('<source>').prop('src', imageFile).prop('type', 'video/mp4')
            );
            const videoContainer = q('<div>').attr('class', 'video-container');
            videoContainer.append(videoElement);

            thumb.append(videoContainerBlurred);
            thumb.append(videoContainer);
          }
          divElement.append(thumb);

          const foot = q('<div>').addClass('choose-bg-foot');
          foot.append(q('<span>').addClass('choose-bg-label').text(firstFile?.name ?? ''));
          const clonedChooseBgButtons = q('.choose-bg-buttons').clone().get(0);
          if (clonedChooseBgButtons) {
            foot.append(clonedChooseBgButtons);
            q(clonedChooseBgButtons)
              .find('.choose-bg-activate-button')
              .addClass('choose-bg-activate-button-checked')
              .attr('aria-pressed', 'true');
          }
          divElement.append(foot);

          wireActivateButtons();
          wireDeleteButtons();
          removeBackgroundFromArray();
          refreshBgCount();
        },
        () => {
          console.error('Failed to download file.');
        }
      );
    }
  });

  wireDeleteButtons();
  wireActivateButtons();
  refreshBgCount();
}
