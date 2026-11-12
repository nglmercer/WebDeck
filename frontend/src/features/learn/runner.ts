import type { Driver, DriveStep } from 'driver.js';
import { tutorialActive } from './tour-state';
import type { Guide, GuideStep } from './guides';
import type { Translator } from '../../lib/translations';
import { tick } from 'svelte';

const escape = (text: string) =>
  text.replace(
    /[&<>"']/g,
    (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]!,
  );

export function visibleTarget(selector: string): HTMLElement | undefined {
  return [...document.querySelectorAll<HTMLElement>(selector)].find(
    (element) =>
      element.getClientRects().length > 0 &&
      getComputedStyle(element).visibility !== 'hidden' &&
      !element.closest('[hidden], [inert]'),
  );
}

export function waitForTarget(
  selector: string,
  signal: AbortSignal,
  timeout = 5000,
): Promise<HTMLElement> {
  return new Promise((resolve, reject) => {
    let frame = 0;
    const observer = new MutationObserver(check);
    const timer = setTimeout(() => finish(new Error('ui_guide_target_missing')), timeout);
    function finish(error?: Error, target?: HTMLElement) {
      observer.disconnect();
      cancelAnimationFrame(frame);
      clearTimeout(timer);
      signal.removeEventListener('abort', abort);
      if (error) reject(error);
      else resolve(target!);
    }
    function abort() {
      finish(new DOMException('Guide cancelled', 'AbortError'));
    }
    function check() {
      cancelAnimationFrame(frame);
      if (signal.aborted) {
        abort();
        return;
      }
      const target = visibleTarget(selector);
      if (target) finish(undefined, target);
      else frame = requestAnimationFrame(check);
    }
    signal.addEventListener('abort', abort, { once: true });
    observer.observe(document.documentElement, {
      childList: true,
      subtree: true,
      attributes: true,
    });
    check();
  });
}

export class GuideRunner {
  private tour: Driver | undefined;
  private controller: AbortController | undefined;
  private guide: Guide | undefined;
  private steps: GuideStep[] = [];
  private index = 0;
  private moving = false;
  private finishing: Promise<void> | undefined;
  private origin: HTMLElement | null = null;

  constructor(
    private options: {
      translate: Translator;
      prepare: (step: GuideStep, signal: AbortSignal) => Promise<void>;
      restore: () => Promise<void>;
      complete: (id: string) => void;
      error: (message: string) => void;
      focus?: () => void;
    },
  ) {}

  async start(guide: Guide, steps = guide.steps, origin: HTMLElement | null = null) {
    await this.stop();
    this.guide = guide;
    this.steps = steps;
    this.index = 0;
    this.origin = origin;
    this.controller = new AbortController();
    tutorialActive.set(true);
    window.addEventListener('keydown', this.keyboard, true);
    for (const event of ['pointerdown', 'click', 'contextmenu'] as const)
      window.addEventListener(event, this.pointer, true);
    await this.go(0);
  }

  private pointer = (event: Event) => {
    if (!event.isTrusted) return;
    const target = event.target instanceof Element ? event.target : null;
    if (target?.closest('.driver-popover, .driver-overlay')) return;
    const step = this.steps[this.index];
    if (!this.moving && step?.interactive && this.tour?.getActiveElement()?.contains(target))
      return;
    event.preventDefault();
    event.stopImmediatePropagation();
  };

  private keyboard = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopImmediatePropagation();
      void this.stop();
      return;
    }
    const popover = document.querySelector<HTMLElement>('.driver-popover');
    const interactive = this.steps[this.index]?.interactive;
    const active = this.tour?.getActiveElement();
    const inExample = interactive && active?.contains(event.target as Node);
    if (['ArrowRight', 'ArrowLeft'].includes(event.key) && !inExample) {
      event.preventDefault();
      event.stopImmediatePropagation();
      void this.go(this.index + (event.key === 'ArrowRight' ? 1 : -1));
      return;
    }
    if (event.key === 'Tab') {
      event.preventDefault();
      event.stopImmediatePropagation();
      const elements = [
        ...(popover?.querySelectorAll<HTMLElement>('button:not(:disabled)') ?? []),
        ...(interactive
          ? (active?.querySelectorAll<HTMLElement>(
              'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled)',
            ) ?? [])
          : []),
        ...(interactive && active instanceof HTMLButtonElement ? [active] : []),
      ].filter((element) => element.getClientRects().length > 0);
      const index = elements.findIndex((element) => element === document.activeElement);
      elements[(index + (event.shiftKey ? -1 : 1) + elements.length) % elements.length]?.focus();
      return;
    }
    if (!popover?.contains(event.target as Node) && !inExample) {
      event.preventDefault();
      event.stopImmediatePropagation();
    }
  };

  private async go(index: number) {
    if (this.moving || !this.guide || !this.controller || index < 0) return;
    if (index >= this.steps.length) {
      await this.stop(true);
      return;
    }
    this.moving = true;
    const controller = this.controller;
    const step = this.steps[index]!;
    try {
      this.setBusy(true);
      this.clearMasks();
      await this.options.prepare(step, controller.signal);
      const target = await waitForTarget(step.target, controller.signal);
      if (controller.signal.aborted) return;
      for (const layer of document.querySelectorAll('dialog[open], .approvals')) {
        if (!layer.contains(target)) layer.setAttribute('data-guide-obscured', '');
      }
      this.index = index;
      if (!this.tour) {
        const { driver } = await import('driver.js');
        if (controller.signal.aborted) return;
        const t = this.options.translate;
        const steps: DriveStep[] = this.steps.map((item) => ({
          element: () => visibleTarget(item.target)!,
          disableActiveInteraction: !item.interactive,
          popover: { title: escape(t(item.title)), description: escape(t(item.description)) },
        }));
        this.tour = driver({
          animate: !matchMedia('(prefers-reduced-motion: reduce)').matches,
          smoothScroll: false,
          showProgress: true,
          allowKeyboardControl: false,
          disableActiveInteraction: true,
          popoverClass: 'webdeck-tour',
          stageRadius: 12,
          nextBtnText: escape(t('ui_next_step')),
          prevBtnText: escape(t('ui_previous_step')),
          doneBtnText: escape(t('ui_finish_tour')),
          progressText: escape(t('ui_tour_progress')),
          onNextClick: () => void this.go(this.index + 1),
          onPrevClick: () => void this.go(this.index - 1),
          onCloseClick: () => void this.stop(),
          onDestroyStarted: () => void this.stop(),
          onPopoverRender: (popover) => {
            popover.closeButton.setAttribute('aria-label', t('ui_close_tour'));
            popover.wrapper.dataset.guide = this.guide!.id;
            popover.wrapper.dataset.guideStep = this.steps[this.index]!.id;
            popover.wrapper.setAttribute('aria-modal', 'true');
          },
          steps,
        });
        this.tour.drive(index);
      } else this.tour.moveTo(index);
      // The prepared target must remain present after Driver.js measures it.
      if (!target.isConnected) throw new Error('ui_guide_target_missing');
      this.tour.refresh();
      document
        .querySelector<HTMLButtonElement>('.driver-popover-next-btn')
        ?.focus({ preventScroll: true });
    } catch (error) {
      if (!controller.signal.aborted) {
        await this.stop();
        this.options.error(error instanceof Error ? error.message : String(error));
      }
    } finally {
      this.moving = false;
      this.setBusy(false);
    }
  }

  private setBusy(busy: boolean) {
    for (const button of document.querySelectorAll<HTMLButtonElement>(
      '.driver-popover-next-btn, .driver-popover-prev-btn',
    )) {
      button.disabled =
        busy || (button.classList.contains('driver-popover-prev-btn') && this.index === 0);
    }
  }

  private clearMasks() {
    for (const element of document.querySelectorAll('[data-guide-obscured]'))
      element.removeAttribute('data-guide-obscured');
  }

  async stop(completed = false): Promise<void> {
    if (this.finishing) return this.finishing;
    if (!this.controller) return;
    const id = this.guide?.id;
    this.controller.abort();
    this.controller = undefined;
    this.tour?.destroy();
    this.tour = undefined;
    window.removeEventListener('keydown', this.keyboard, true);
    for (const event of ['pointerdown', 'click', 'contextmenu'] as const)
      window.removeEventListener(event, this.pointer, true);
    this.finishing = (async () => {
      try {
        await this.options.restore();
      } finally {
        this.clearMasks();
        tutorialActive.set(false);
        await tick();
        if (this.origin?.isConnected && this.origin.getClientRects().length)
          this.origin.focus({ preventScroll: true });
        else this.options.focus?.();
        this.guide = undefined;
        this.steps = [];
        this.moving = false;
        this.finishing = undefined;
      }
      if (completed && id) this.options.complete(id);
    })();
    return this.finishing;
  }
}
