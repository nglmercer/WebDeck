<script lang="ts">
  import EditorStyle from '../components/EditorStyle.svelte';
  import CloseIcon from '../components/CloseIcon.svelte';
  import { text } from '../framework/i18n';
  import type { BootContext, JsonObject } from '../framework/types';
  import ArgsBlock from './ArgsBlock.svelte';
  import { editModalData } from './editmodal';

  interface Props {
    ctx: BootContext;
    editModalId: string;
    entry: JsonObject;
    message: string;
  }

  let { ctx, editModalId, entry, message }: Props = $props();

  // Render-once by design: the grid mounts one modal per button from a
  // single boot context, so this intentionally captures the initial props.
  // svelte-ignore state_referenced_locally
  const modal = editModalData(ctx, editModalId, entry, message);
</script>

<div
  class="editbutton-modal-container {modal.dark}"
  id="edit-modal-container-{modal.modalId}"
  edit_modal_ID={modal.modalId}
>
  <div class="editbutton-modal-content {modal.dark}">
    <div class="editbutton-modal-header bold edit-modal-container-{modal.modalId}">
      <h1 class="editbutton-modal"> {text('configure_your_button')} </h1>
      <div class="editbutton-modal-close">
        <CloseIcon cls="editbutton-config-modal" dark={modal.dark} />
      </div>
    </div>
    <div class="editbutton-modal-main">
      <div class="config-container {modal.dark}">
        <form class="args-form" edit_modal_ID={modal.modalId} novalidate>
          {#if modal.args !== null}
            <ArgsBlock
              ctx={ctx}
              category={modal.args.category}
              command={modal.args.command}
              subId={modal.args.subId}
              parentCommand={modal.args.parentCommand}
              commandValue={modal.args.commandValue}
              modalId={modal.modalId}
              idAttr="edit_modal_ID"
              prefill={modal.args.prefill}
            />
            <div class="editorStyle-bar {modal.dark}"></div>
          {/if}
          <EditorStyle
            dark={modal.dark}
            id={modal.modalId}
            preview={modal.preview}
            defaultSize={modal.defaultSize}
            backgroundColor={modal.backgroundColor}
            buttonName={modal.buttonName}
            nameValue={modal.nameValue}
          />
          {#if modal.showDevbox}
            <div class="editorStyle-bar {modal.dark}"></div>
            <div class="arg_container" edit_modal_ID={modal.modalId}>
              <label for="command_{modal.modalId}"> {text('edit_command')} </label>
              <input
                class={modal.dark}
                type="text"
                name=""
                id="command_{modal.modalId}"
                value={modal.devboxValue}
              />
            </div>
          {/if}
          <input
            type="submit"
            id="{modal.modalId}_submit"
            class="modal-button save-config {modal.dark}"
            style="margin-top: 30px;"
            value={text('save')}
          />
        </form>
      </div>
    </div>
  </div>
</div>
