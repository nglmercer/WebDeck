<script lang="ts">
  import { text } from '../framework/i18n';
  import { NAMED_KEYS, normalizeCapturedKey } from './keyfield';
  import { wireSearchDropdown } from './search-dropdown';

  /**
   * Key-selector field interior (see `keyfield.ts`): one collected text
   * input plus capture button and named-keys search list. Search wiring
   * runs in an effect (async, post-mount): this element upgrades inside
   * modal subtrees (nested mounts sharing the outer batch), so nothing
   * here may flush synchronously during connect.
   */

  interface Props {
    /** Modal id suffix (unique per modal). */
    fieldId: string;
    dark: string;
    initialValue: string;
  }

  let { fieldId, dark, initialValue }: Props = $props();
  let inputEl: HTMLInputElement | null = $state(null);
  let armed = $state(false);

  const listId = $derived(`key-list_${fieldId}`);

  $effect(() => {
    wireSearchDropdown(listId, NAMED_KEYS, (value) => {
      if (inputEl) inputEl.value = value;
    });
  });

  $effect(() => {
    return () => document.removeEventListener('keydown', onKeydown, true);
  });

  function disarm(): void {
    armed = false;
    document.removeEventListener('keydown', onKeydown, true);
  }

  function onKeydown(event: Event): void {
    const e = event as KeyboardEvent;
    e.preventDefault();
    e.stopPropagation();
    disarm();
    const name = normalizeCapturedKey(e.key);
    if (name !== null && inputEl) inputEl.value = name;
  }

  function onCaptureClick(): void {
    if (armed) {
      disarm();
      return;
    }
    armed = true;
    document.addEventListener('keydown', onKeydown, true);
  }
</script>

<div class="key-field">
  <div class="key-field-row">
    <input
      class={dark}
      type="text"
      name=""
      size="10"
      id="key-input_{fieldId}"
      value={initialValue !== '' ? initialValue : undefined}
      bind:this={inputEl}
    />
    <button
      type="button"
      class="key-capture {dark}"
      class:arming={armed}
      id="key-capture_{fieldId}"
      onclick={onCaptureClick}>{armed ? text('key_capture_prompt') : text('key_capture')}</button
    >
  </div>
  <search-dropdown
    class={dark}
    id={listId}
    placeholder={text('key_search_keys')}
    input-class="key-aux"
  ></search-dropdown>
</div>
