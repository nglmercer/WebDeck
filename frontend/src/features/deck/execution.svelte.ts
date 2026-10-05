import { createTranslator, type Translator } from '../../lib/translations';
import { capabilityNames } from '../../lib/capability-labels';
const english = createTranslator(() => ({}));
import type { Button, Command, CommandEvent, DeckBoot } from '../../lib/contracts';
import { execute } from '../../lib/api/client';
export function permissionReason(
  button: Button,
  deck: DeckBoot | null,
  editing: boolean,
  t: Translator = english,
): string {
  if (editing) return '';
  if (button.action.type === 'settings' || button.action.type === 'edit')
    return deck?.can_edit ? '' : t('This device has no deck editing permission.');
  if (button.action.type === 'command') {
    const capability = deck?.button_capabilities[button.id];
    if (!capability) return t('This action is unavailable. Reload the deck.');
    if (!deck?.capabilities.includes(capability))
      return t('ui_device_needs_permission', {
        permission: t(capabilityNames[capability] ?? capability),
      });
  }
  return '';
}
export class Execution {
  running = $state<Record<string, boolean>>({});
  states = $state<Record<string, 'completed' | 'failed'>>({});
  messages = $state<Record<string, string>>({});
  private timers = new Map<string, ReturnType<typeof setTimeout>>();
  private disposed = false;
  constructor(
    private runner: (
      command: Command,
      transport: 'http' | 'socket',
    ) => Promise<CommandEvent> = execute,
  ) {}
  async run(button: Button, transport: 'http' | 'socket') {
    if (this.disposed || this.running[button.id]) return;
    clearTimeout(this.timers.get(button.id));
    this.running[button.id] = true;
    delete this.states[button.id];
    delete this.messages[button.id];
    try {
      const event = await this.runner({ type: 'button', button_id: button.id }, transport);
      if (event.state === 'failed') throw new Error(event.message);
      if (this.disposed) return;
      this.states[button.id] = 'completed';
      this.timers.set(
        button.id,
        setTimeout(() => {
          delete this.states[button.id];
          this.timers.delete(button.id);
        }, 1800),
      );
    } catch (error) {
      if (!this.disposed) {
        this.states[button.id] = 'failed';
        this.messages[button.id] = error instanceof Error ? error.message : String(error);
      }
      throw error;
    } finally {
      if (!this.disposed) this.running[button.id] = false;
    }
  }
  dispose() {
    this.disposed = true;
    this.timers.forEach(clearTimeout);
    this.timers.clear();
  }
}
