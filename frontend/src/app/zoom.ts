import { showError } from './toast';

// Fullscreen / zoom / grid sizing (index.jinja tail block).
// NOTE: upstream declares `scale` inside the zoom-in try-block, so the
// zoom-out handler throws ReferenceError on click; one shared variable
// keeps both buttons working (the obvious intent).

let scale = 1;

export function goFullscreen(): void {
  const doc = document as Document & {
    mozFullScreenElement?: Element;
    webkitFullscreenElement?: Element;
    msFullscreenElement?: Element;
  };
  const docEl = document.documentElement as HTMLElement & {
    mozRequestFullScreen?: () => void;
    webkitRequestFullscreen?: () => void;
    msRequestFullscreen?: () => void;
  };
  if (
    !document.fullscreenElement &&
    !doc.mozFullScreenElement &&
    !doc.webkitFullscreenElement &&
    !doc.msFullscreenElement
  ) {
    if (docEl.requestFullscreen) {
      docEl.requestFullscreen();
    } else if (docEl.msRequestFullscreen) {
      docEl.msRequestFullscreen();
    } else if (docEl.mozRequestFullScreen) {
      docEl.mozRequestFullScreen();
    } else if (docEl.webkitRequestFullscreen) {
      docEl.webkitRequestFullscreen();
    }
  } else {
    const exit = document as Document & {
      mozCancelFullScreen?: () => void;
      webkitExitFullscreen?: () => void;
      msExitFullscreen?: () => void;
    };
    if (document.exitFullscreen) {
      document.exitFullscreen();
    } else if (exit.msExitFullscreen) {
      exit.msExitFullscreen();
    } else if (exit.mozCancelFullScreen) {
      exit.mozCancelFullScreen();
    } else if (exit.webkitExitFullscreen) {
      exit.webkitExitFullscreen();
    }
  }
}

export function auto_resize(): void {
  const divs = document.querySelectorAll('.buttons-center');
  let div: Element | undefined;
  for (let i = 0; i < divs.length; i++) {
    if (!divs[i].classList.contains('invisible')) {
      div = divs[i];
      break;
    }
  }
  if (!div) return;
  const target = div;
  let currentZoom = document.body.offsetWidth / 100;

  function applyZoom(newZoom: number): void {
    currentZoom = newZoom;
    const scalePercentage = 100 / currentZoom;
    document.body.style.transformOrigin = 'left top';
    document.body.style.transform = `scale(${currentZoom})`;
    document.body.style.width = `${scalePercentage}%`;
    document.body.style.height = `${scalePercentage}%`;
  }

  let count = 0;
  while (count < 1000) {
    count++;
    const rect = target.getBoundingClientRect();
    if (!(rect.right > window.innerWidth || rect.bottom > window.innerHeight)) {
      currentZoom += 0.05;
      applyZoom(currentZoom);
    } else {
      count = 1000;
      break;
    }
  }

  count = 0;
  while (count < 1000) {
    count++;
    const rect = target.getBoundingClientRect();
    if (rect.right > window.innerWidth || rect.bottom > window.innerHeight) {
      currentZoom -= 0.05;
      applyZoom(currentZoom);
    } else {
      count = 1000;
      break;
    }
  }
}

export function wireZoomControls(isSwapMode: () => boolean, frontWidth: string, frontHeight: string): void {
  const fullscreenBtn = document.querySelector('.fullscreen-btn');
  const zoomInBtn = document.querySelector('.zoom-in-btn');
  const zoomOutBtn = document.querySelector('.zoom-out-btn');
  const buttonsCenter = document.querySelector('.buttons-center');
  const shrinkBtn = document.querySelector('.shrink-btn');
  const expandBtn = document.querySelector('.expand-btn');
  const dezoomBtn = document.querySelector('.dezoom-btn');
  const zoomBtn = document.querySelector('.zoom-btn');

  if (fullscreenBtn) {
    fullscreenBtn.addEventListener('click', () => {
      if (document.fullscreenEnabled && !isSwapMode()) {
        if (document.fullscreenElement) {
          document.exitFullscreen();
        } else {
          goFullscreen();
        }
        auto_resize();
      } else {
        showError('Fullscreen is not supported on your device.');
      }
    });
  }

  try {
    zoomInBtn?.addEventListener('click', () => {
      if (!isSwapMode()) {
        scale += 0.1;
        document.documentElement.style.transform = `scale(${scale})`;
      }
    });
  } catch {
    // empty
  }

  try {
    zoomOutBtn?.addEventListener('click', () => {
      if (!isSwapMode()) {
        scale -= 0.1;
        document.documentElement.style.transform = `scale(${scale})`;
      }
    });
  } catch {
    // empty
  }

  try {
    shrinkBtn?.addEventListener('click', () => {
      if (!isSwapMode() && buttonsCenter) {
        const currentMargin = parseInt(getComputedStyle(buttonsCenter).marginLeft);
        (buttonsCenter as HTMLElement).style.marginLeft = `${currentMargin + 1}px`;
        (buttonsCenter as HTMLElement).style.marginRight = `${currentMargin + 1}px`;
      }
    });
  } catch {
    // empty
  }

  try {
    expandBtn?.addEventListener('click', () => {
      if (!isSwapMode() && buttonsCenter) {
        const currentMargin = parseInt(getComputedStyle(buttonsCenter).marginLeft);
        (buttonsCenter as HTMLElement).style.marginLeft = `${currentMargin - 1}px`;
        (buttonsCenter as HTMLElement).style.marginRight = `${currentMargin - 1}px`;
      }
    });
  } catch {
    // empty
  }

  const maxRows = parseInt(frontHeight);
  const maxCols = parseInt(frontWidth);

  const smallDiv = document.querySelector('.form-0');
  if (smallDiv) {
    const smallDivStyles = getComputedStyle(smallDiv);
    const smallDivWidth =
      parseInt(smallDivStyles.width) + parseInt(smallDivStyles.paddingLeft) + parseInt(smallDivStyles.paddingRight);
    const smallDivHeight =
      parseInt(smallDivStyles.height) + parseInt(smallDivStyles.paddingTop) + parseInt(smallDivStyles.paddingBottom);

    const maxWidth = maxCols * smallDivWidth + 10;
    const maxHeight = maxCols * smallDivHeight + 10;

    const bigDivs = document.querySelectorAll('[id^="folder-"].buttons-center');
    bigDivs.forEach((bigDiv) => {
      (bigDiv as HTMLElement).style.maxWidth = `${maxWidth}px`;
      (bigDiv as HTMLElement).style.maxHeight = `${maxHeight}px`;
    });
  }
  void maxRows;

  let currentZoom = 1;

  try {
    dezoomBtn?.addEventListener('click', () => {
      if (!isSwapMode()) {
        currentZoom = currentZoom - 0.05;
        const calcWidth = 100 / currentZoom;
        const calcHeight = 100 / currentZoom;
        document.body.style.transformOrigin = 'left top';
        document.body.style.transform = 'scale(' + currentZoom + ')';
        document.body.style.width = `${calcWidth}%`;
        document.body.style.height = `${calcHeight}%`;
      }
    });
  } catch {
    // empty
  }

  try {
    zoomBtn?.addEventListener('click', () => {
      if (!isSwapMode()) {
        currentZoom = currentZoom + 0.05;
        const calcWidth = 100 / currentZoom;
        const calcHeight = 100 / currentZoom;
        document.body.style.transformOrigin = 'left top';
        document.body.style.transform = 'scale(' + currentZoom + ')';
        document.body.style.width = `${calcWidth}%`;
        document.body.style.height = `${calcHeight}%`;
      }
    });
  } catch {
    // empty
  }

  addEventListener('resize', () => {
    // empty
  });
  onresize = () => {
    auto_resize();
  };
}
