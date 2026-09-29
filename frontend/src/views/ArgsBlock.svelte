<script lang="ts">
  import { pickFilePath, pickFolderPath, uploadFile } from '../api/uploads';
  import KeyValueEditor from '../components/KeyValueEditor.svelte';
  import { stringAttrs } from '../components/string-attrs';
  import { text } from '../framework/i18n';
  import type { BootContext, JsonObject } from '../framework/types';
  import {
    argsData,
    registerShowArg,
    type ArgsData,
    type FieldData,
    type ModalIdAttr,
    type ArgsPrefill,
    type VisibleWhen,
  } from './args';

  /**
   * The `.args-container` block (shared add/edit template): one branch per
   * arg with label/group chrome. Prefill/numbering semantics come from the
   * shared `argsData` traversal; choice-pane switching registers the same
   * `showArg_*` global the string renderer relied on.
   */

  interface Props {
    ctx: BootContext;
    category: string;
    command: string;
    subId: number;
    parentCommand: string;
    commandValue: JsonObject;
    modalId: string;
    idAttr: ModalIdAttr;
    prefill?: ArgsPrefill | undefined;
  }

  let {
    ctx,
    category,
    command,
    subId,
    parentCommand,
    commandValue,
    modalId,
    idAttr,
    prefill,
  }: Props = $props();

  // Render-once by design: modals mount a single context, so this
  // intentionally captures the initial prop values.
  // svelte-ignore state_referenced_locally
  const data: ArgsData = argsData({
    ctx,
    category,
    command,
    subId,
    parentCommand,
    commandValue,
    modalId,
    idAttr,
    ...(prefill !== undefined ? { cursor: { prefill, pos: 0 } } : {}),
  });

  $effect(() => {
    registerShowArg(modalId, idAttr);
  });

  let root: HTMLElement | undefined = $state();
  let hiddenArgs = $state<Set<number>>(new Set());

  /** Controller value for a `visibleWhen` rule (fail-open: undefined). */
  function controllerValue(argIndex: number): string | undefined {
    const container = root?.querySelector(`[data-branch="input"][arg_id="${argIndex}"]`);
    const field = container?.querySelector('select, input, textarea');
    if (field instanceof HTMLSelectElement) return field.value;
    if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) {
      return field.value;
    }
    return undefined;
  }

  function ruleVisible(rule: VisibleWhen): boolean {
    const value = controllerValue(rule.arg);
    if (value === undefined) return true;
    if (rule.in !== undefined && !rule.in.includes(value)) return false;
    if (rule.notIn !== undefined && rule.notIn.includes(value)) return false;
    return true;
  }

  /** Re-evaluate conditional branches (mount + any field change). Hidden
   * branches submit nothing: collection already skips `display: none`. */
  function refreshVisibility(): void {
    const hidden = new Set<number>();
    for (const branch of data.branches) {
      if (
        branch.kind === 'input' &&
        branch.visibleWhen !== undefined &&
        !ruleVisible(branch.visibleWhen)
      ) {
        hidden.add(branch.argIndex);
      }
    }
    hiddenArgs = hidden;
  }

  $effect(() => {
    refreshVisibility();
  });

  /** Dynamic modal-id attribute (`arg_modal_ID` vs `edit_modal_ID`). */
  function idAttrs(): Record<string, string> {
    return { [data.idAttr]: data.modalId };
  }

  function choiceAttrs(choiceIndex: number): Record<string, string> {
    return { onchange: `showArg_${data.modalId}('${choiceIndex}')` };
  }

  /** Digit scrub for non-negative number fields (first match only, 1:1). */
  function scrubPositive(event: Event): void {
    const el = event.currentTarget as HTMLInputElement;
    el.value = el.value.replace(/[^0-9]/g, '');
  }

  /** Sync select value (happy-dom ignores the `selected` attribute). */
  function syncSelect(node: HTMLSelectElement, value: string | undefined): void {
    if (value === undefined) return;
    if ([...node.options].some((o) => o.value === value)) node.value = value;
  }

  function dropdownSelected(field: Extract<FieldData, { kind: 'dropdown' }>): string | undefined {
    return field.options.find((o) => o.selected)?.id;
  }

  function gpusSelected(field: Extract<FieldData, { kind: 'gpus' }>): string | undefined {
    return field.options.find((o) => o.selected)?.key;
  }

  function diskSelected(field: Extract<FieldData, { kind: 'diskLetter' }>): string | undefined {
    return field.options.find((o) => o.selected)?.disk;
  }

  /** Fill the row's path input from the native picker (empty = cancelled). */
  async function fillFromPicker(target: HTMLInputElement | null, picked: Promise<string>): Promise<void> {
    if (!target) return;
    try {
      const path = await picked;
      if (path !== '') target.value = path;
    } catch (error) {
      console.error('Error during request:', error);
    }
  }

  function rowInput(button: HTMLElement, selector: string): HTMLInputElement | null {
    return button.parentElement?.querySelector(selector) as HTMLInputElement | null;
  }

  function pickFile(event: Event, filetypes: string): void {
    const button = event.currentTarget as HTMLButtonElement;
    void fillFromPicker(rowInput(button, 'input.filepath'), pickFilePath(filetypes));
  }

  function pickFolder(event: Event): void {
    const button = event.currentTarget as HTMLButtonElement;
    void fillFromPicker(rowInput(button, 'input.folderpath'), pickFolderPath());
  }

  function uploadAudio(event: Event): void {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const form = new FormData();
    form.append('file', file);
    void uploadFile(form).then(undefined, () => {
      console.error('Failed to download file.');
    });
  }
</script>
{#snippet argField(field: FieldData)}
  {#if field.kind === 'foldername'}
    <div class="webdeck_foldername_ALL">
      {#each field.folders as folder}
        <div class="webdeck_foldername">
          <input
            checked={field.checked !== undefined && folder === field.checked}
            class={data.dark}
            type="radio"
            name="file"
            value={folder}
          />
          <!-- svelte-ignore a11y_label_has_associated_control: 1:1 port, upstream for-names dangle. -->
          <label for={folder}>{folder}</label>
        </div>
      {/each}
    </div>
  {:else if field.kind === 'audioUpload'}
    <input
      class="{data.dark} audio-input"
      id="audio-input_{data.modalId}"
      type="file"
      name="file"
      accept=".mp3"
      data-preserved={field.preserved}
      onchange={uploadAudio}
    />
  {:else if field.kind === 'filetype'}
    {#each field.inputs as input}
      <input
        class="{data.dark} audio-input"
        id="audio-input_{data.modalId}"
        type="file"
        name="file"
        accept={input.accepts}
        data-preserved={input.preserved}
        onchange={uploadAudio}
      />
    {/each}
  {:else if field.kind === 'filepath'}
    {#each field.inputs as input}
      <div class="filepath">
        <button class="filepath" filetypes={input.filetypes} onclick={(e) => pickFile(e, input.filetypes)}>
          {text('select_your_file')}
        </button>
        <input
          type="text"
          class="filepath {data.dark}"
          placeholder={text('no_file_chosen')}
          value={input.value}
        />
      </div>
    {/each}
  {:else if field.kind === 'filePicker'}
    <div class="filepath">
      <button class="filepath" onclick={(e) => pickFile(e, '')}> {text('select_your_file')} </button>
      <input
        type="text"
        class="filepath {data.dark}"
        placeholder={text('no_file_chosen')}
        value={field.value}
      />
    </div>
  {:else if field.kind === 'folderPicker'}
    <div class="folderpath">
      <button class="folderpath" onclick={pickFolder}> {text('select_your_file')} </button>
      <input
        type="text"
        class="folderpath {data.dark}"
        placeholder={text('no_file_chosen')}
        value={field.value}
      />
    </div>
  {:else if field.kind === 'url'}
    <input
      class={data.dark}
      type="url"
      name=""
      id="url_{data.modalId}"
      placeholder="https://example.com"
      value={field.value}
    />
  {:else if field.kind === 'key'}
    <key-field
      id="key-field_{data.modalId}"
      field-id={data.modalId}
      dark={data.dark}
      value={field.value}
    ></key-field>
  {:else if field.kind === 'number'}
    {#each field.inputs as input}
      {#if input.min.startsWith('-') || input.max.startsWith('-')}
        <input
          class={data.dark}
          type="number"
          name=""
          min={input.min}
          max={input.max}
          placeholder={field.placeholder}
          value={input.value}
        />
      {:else}
        <input
          class={data.dark}
          type="number"
          pattern="[0-9]*"
          oninput={scrubPositive}
          name=""
          min={input.min}
          max={input.max}
          placeholder={field.placeholder}
          value={input.value}
        />
      {/if}
    {/each}
  {:else if field.kind === 'longtext'}
    <textarea class={data.dark} name="" rows="5" cols="33">{field.value ?? ''}</textarea>
  {:else if field.kind === 'headers'}
    <KeyValueEditor initial={field.rows} dark={data.dark} />
  {:else if field.kind === 'usageTitle'}
    <input
      id="usage-title-input_{data.modalId}"
      class={data.dark}
      type="text"
      name=""
      size="10"
      value={field.preset !== '' ? field.preset : undefined}
    />
  {:else if field.kind === 'text'}
    <input
      class={data.dark}
      type="text"
      name=""
      size="10"
      value={field.preset !== '' ? field.preset : undefined}
    />
  {:else if field.kind === 'hidden'}
    <input class="invisible" type="text" size="10" value={field.value} />
  {:else if field.kind === 'dropdown'}
    <select name="" use:syncSelect={dropdownSelected(field)}>
      {#each field.options as option}
        <option value={option.id} selected={option.selected}> {option.label} </option>
      {/each}
    </select>
  {:else if field.kind === 'gpus'}
    <select name="" use:syncSelect={gpusSelected(field)}>
      {#each field.options as option}
        <option value={option.key} selected={option.selected}> {option.label} </option>
      {/each}
    </select>
  {:else if field.kind === 'diskLetter'}
    <select id="disk-letter_{data.modalId}" name="" use:syncSelect={diskSelected(field)}>
      {#each field.options as option}
        <option value={option.disk} selected={option.selected}> {option.disk} </option>
      {/each}
    </select>
  {/if}
{/snippet}



<div
  class="args-container {data.dark}"
  use:stringAttrs={idAttrs()}
  bind:this={root}
  onchange={refreshVisibility}
  oninput={refreshVisibility}
>
  {#each data.branches as branch}
    {#if branch.kind === 'input'}
      <div
        class="arg_container"
        data-branch="input"
        use:stringAttrs={idAttrs()}
        arg_id={String(branch.argIndex)}
        style={hiddenArgs.has(branch.argIndex) ? 'display: none;' : undefined}
      >
        <!-- svelte-ignore a11y_label_has_associated_control: 1:1 port, upstream for-ids rarely exist. -->
        <label for="{branch.label}_{data.modalId}">{branch.label}:</label>
        {@render argField(branch.field)}
      </div>
      {#if branch.folderForm}
        <div class="webdeck_foldername_div">
          <form id="webdeck_foldername_form" novalidate>
            <input
              class={data.dark}
              type="text"
              id="folderName_{data.modalId}"
              name="folderName"
              placeholder="New folder name"
            />
            <button id="submitButton_{data.modalId}" type="submit">Create folder</button>
          </form>
        </div>
      {/if}
    {:else if branch.kind === 'choice'}
      <div class="choices_ALL">
        <!-- svelte-ignore a11y_label_has_associated_control: 1:1 port, upstream for="choice" dangles. -->
        <label for="choice">{text('choose_option')} :</label><br />
        {#each branch.options as option}
          <div class="choice">
            <input
              checked={option.selected}
              class="choice {data.dark}"
              type="radio"
              name="choice"
              value={String(option.choiceIndex)}
              use:stringAttrs={choiceAttrs(option.choiceIndex)}
            />
            <!-- svelte-ignore a11y_label_has_associated_control: 1:1 port, upstream for-names dangle. -->
            <label for={option.name}>{option.name}</label>
          </div>
          <div
            style={option.selected ? undefined : 'display: none;'}
            class="arg_container"
            use:stringAttrs={idAttrs()}
            arg_id={String(option.choiceIndex)}
          >
            {#each option.fields as field}
              {@render argField(field)}
            {/each}
          </div>
        {/each}
      </div>
    {:else if branch.kind === 'hidden'}
      {@render argField(branch.field)}
    {/if}
  {/each}
</div>
