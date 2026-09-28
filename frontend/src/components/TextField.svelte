<script lang="ts">
  /** Labeled text input, optionally a password field with visibility toggle. */
  interface Props {
    dark: string;
    cls: string;
    label?: string | undefined;
    labelFor?: string | undefined;
    id: string;
    name: string;
    value: string;
    password?: boolean | undefined;
    toggleId?: string | undefined;
    placeholder?: string | undefined;
  }

  let { dark, cls, label, labelFor, id, name, value, password, toggleId, placeholder }: Props =
    $props();

  const hasValue = $derived(value.trim() !== '');
  const showToggle = $derived(password === true && toggleId !== undefined);
  const inputClass = $derived(`${cls} ${dark}`);
</script>

<div class="setting wd2-field {cls}-wrap">
  {#if label !== undefined}<label class="wd2-label" for={labelFor ?? id}> {label} </label>{/if}{#if showToggle}<div class="password-container"><input class={inputClass} type="password" {id} {name} value={hasValue ? value : undefined} {placeholder} /><!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions: 1:1 port, upstream wires a bare onclick span. --><span id={toggleId} class="show-password" onclick={() => window.togglePasswordVisibility?.(id, toggleId ?? '')}></span></div>{:else}<input class={inputClass} type={password === true ? 'password' : 'text'} {id} {name} value={hasValue ? value : undefined} {placeholder} />{/if}
</div>
