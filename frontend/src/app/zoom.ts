import { q, byId } from '../query';
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

/** Last applied fit, reused by the manual zoom buttons. */
let fitState: { s: number; tx: number; ty: number } = { s: 1, tx: 0, ty: 0 };

function applyFit(scaler: HTMLElement, s: number, tx: number, ty: number): void {
  fitState = { s, tx, ty };
  q(scaler).css({ transformOrigin: '0 0', transform: `translate(${tx}px, ${ty}px) scale(${s})` });
}

export function auto_resize(): void {
  const scaler = byId<HTMLElement>('deck-scale').get(0) ?? null;
  if (!scaler) return;
  const div =
    q('.buttons-center')
      .toArray()
      .find((candidate) => !q(candidate).hasClass('invisible')) ?? null;
  // Measure the content box (.all-buttons), not the viewport-wide
  // container: in narrow windows the content overflows the container and
  // the container rect would under-measure it.
  const content = q(div).find('.all-buttons').get(0) ?? null;
  if (!content) return;
  // Reset so the measurement is unscaled. getBoundingClientRect is the
  // transformed bbox, so a portrait-rotated grid measures rotated
  // (possibly with negative offsets) and the fit below compensates.
  q(scaler).css('transform', 'none');
  const rect = content.getBoundingClientRect();
  const w = rect.width;
  const h = rect.height;
  if (!(w > 0) || !(h > 0)) return;
  // Same intent as the upstream grow/shrink loop (fill as much as fits)
  // computed directly: exact, instant, and unable to miss convergence.
  // translate() then centers the scaled box horizontally and pins it to
  // the top, which also pulls rotated overflow back on-screen.
  const vw = window.innerWidth;
  const s = Math.min(vw / w, window.innerHeight / h);
  if (!isFinite(s) || s <= 0) return;
  const tx = (vw - s * w) / 2 - s * rect.x;
  const ty = 0 - s * rect.y;
  applyFit(scaler, s, tx, ty);
}

export function wireZoomControls(isSwapMode: () => boolean, frontWidth: string, frontHeight: string): void {
  const fullscreenBtn = q('.fullscreen-btn').get(0) ?? null;
  const zoomInBtn = q('.zoom-in-btn').get(0) ?? null;
  const zoomOutBtn = q('.zoom-out-btn').get(0) ?? null;
  const buttonsCenter = q('.buttons-center').get(0) ?? null;
  const shrinkBtn = q('.shrink-btn').get(0) ?? null;
  const expandBtn = q('.expand-btn').get(0) ?? null;
  const dezoomBtn = q('.dezoom-btn').get(0) ?? null;
  const zoomBtn = q('.zoom-btn').get(0) ?? null;

  q(fullscreenBtn).on('click', () => {
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

  try {
    q(zoomInBtn).on('click', () => {
      if (!isSwapMode()) {
        scale += 0.1;
        q(document.documentElement).css('transform', `scale(${scale})`);
      }
    });
  } catch {
    // empty
  }

  try {
    q(zoomOutBtn).on('click', () => {
      if (!isSwapMode()) {
        scale -= 0.1;
        q(document.documentElement).css('transform', `scale(${scale})`);
      }
    });
  } catch {
    // empty
  }

  try {
    q(shrinkBtn).on('click', () => {
      if (!isSwapMode() && buttonsCenter) {
        const currentMargin = parseInt(q(buttonsCenter).css('marginLeft') ?? '');
        q(buttonsCenter).css({
          marginLeft: `${currentMargin + 1}px`,
          marginRight: `${currentMargin + 1}px`,
        });
      }
    });
  } catch {
    // empty
  }

  try {
    q(expandBtn).on('click', () => {
      if (!isSwapMode() && buttonsCenter) {
        const currentMargin = parseInt(q(buttonsCenter).css('marginLeft') ?? '');
        q(buttonsCenter).css({
          marginLeft: `${currentMargin - 1}px`,
          marginRight: `${currentMargin - 1}px`,
        });
      }
    });
  } catch {
    // empty
  }

  const maxRows = parseInt(frontHeight);
  const maxCols = parseInt(frontWidth);

  const smallDiv = q('.form-0').get(0) ?? null;
  if (smallDiv) {
    const smallDivWidth =
      parseInt(q(smallDiv).css('width') ?? '') +
      parseInt(q(smallDiv).css('paddingLeft') ?? '') +
      parseInt(q(smallDiv).css('paddingRight') ?? '');
    const smallDivHeight =
      parseInt(q(smallDiv).css('height') ?? '') +
      parseInt(q(smallDiv).css('paddingTop') ?? '') +
      parseInt(q(smallDiv).css('paddingBottom') ?? '');

    const maxWidth = maxCols * smallDivWidth + 10;
    const maxHeight = maxCols * smallDivHeight + 10;

    q('[id^="folder-"].buttons-center')
      .toArray()
      .forEach((bigDiv) => {
        q(bigDiv).css({ maxWidth: `${maxWidth}px`, maxHeight: `${maxHeight}px` });
      });
    // Pin the column count on the content box too: without this a narrow
    // window squeezes .all-buttons and rewraps the grid instead of
    // scaling it (upstream only constrained the outer container).
    q('[id^="folder-"].all-buttons').css('maxWidth', `${maxWidth}px`);
  }
  void maxRows;

  const scaler = byId<HTMLElement>('deck-scale').get(0) ?? null;

  // Manual zoom steps relative to the current auto fit (upstream reset
  // to an absolute zoom of 1 here); translation is preserved so the
  // grid does not jump, and the scale stays positive.
  try {
    q(dezoomBtn).on('click', () => {
      if (!isSwapMode() && scaler) {
        applyFit(scaler, Math.max(0.05, fitState.s - 0.05), fitState.tx, fitState.ty);
      }
    });
  } catch {
    // empty
  }

  try {
    q(zoomBtn).on('click', () => {
      if (!isSwapMode() && scaler) {
        applyFit(scaler, fitState.s + 0.05, fitState.tx, fitState.ty);
      }
    });
  } catch {
    // empty
  }

  // NOTE: window-level resize listeners have no qdom equivalent
  // (Q wraps elements only), so these stay native.
  addEventListener('resize', () => {
    // empty
  });
  onresize = () => {
    auto_resize();
  };
}
