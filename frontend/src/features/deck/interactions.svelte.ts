export class DeckInteractions {
  controls = $state(false);
  help = $state(false);
  private timer: ReturnType<typeof setTimeout> | undefined;
  private point: { x: number; y: number } | undefined;
  constructor(
    private actions: {
      enabled: () => boolean;
      hasDialog: () => boolean;
      closeEditor: () => void;
      toggleEdit: () => void;
      settings: () => void;
      back: () => void;
    },
  ) {}
  keyboard = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      if (this.actions.hasDialog()) return;
      this.controls = false;
      this.help = false;
      this.actions.closeEditor();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && event.key === ',' && !this.actions.hasDialog()) {
      event.preventDefault();
      this.actions.settings();
      return;
    }
    if (
      event.target instanceof HTMLElement &&
      event.target.closest('input,textarea,select,[contenteditable]')
    )
      return;
    if (this.actions.hasDialog() || event.repeat) return;
    if (event.key === 'F1') {
      event.preventDefault();
      this.help = !this.help;
      return;
    }
    if (this.help) return;
    if (!event.ctrlKey && !event.metaKey && !event.altKey && event.key.toLowerCase() === 'q') {
      event.preventDefault();
      this.actions.toggleEdit();
    }
    if (event.altKey && event.key === 'ArrowLeft') {
      event.preventDefault();
      this.actions.back();
    }
  };
  context = (event: MouseEvent) => {
    if (
      !this.actions.enabled() ||
      this.actions.hasDialog() ||
      !(event.target instanceof Element) ||
      event.target.closest('input,textarea,select,[contenteditable],.editor-toolbar')
    )
      return;
    event.preventDefault();
    this.controls = true;
  };
  startHold = (event: PointerEvent) => {
    if (
      event.pointerType !== 'touch' ||
      !this.actions.enabled() ||
      this.actions.hasDialog() ||
      !(event.target instanceof Element) ||
      !event.target.closest('.deck-grid')
    )
      return;
    this.stopHold();
    this.point = { x: event.clientX, y: event.clientY };
    this.timer = setTimeout(() => {
      this.controls = true;
    }, 600);
  };
  stopHold = () => {
    if (this.timer) clearTimeout(this.timer);
    this.timer = undefined;
    this.point = undefined;
  };
  moveHold = (event: PointerEvent) => {
    if (this.point && Math.hypot(event.clientX - this.point.x, event.clientY - this.point.y) > 8)
      this.stopHold();
  };
  dispose() {
    this.stopHold();
  }
}
