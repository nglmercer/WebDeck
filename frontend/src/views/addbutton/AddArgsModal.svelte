<script lang="ts">
  import EditorStyle from '../../components/EditorStyle.svelte';
  import CloseIcon from '../../components/CloseIcon.svelte';
  import { text } from '../../framework/i18n';
  import type { BootContext } from '../../framework/types';
  import ArgsBlock from '../ArgsBlock.svelte';
  import { addArgsData } from './argsmodal';
  import type { AddModalContext } from './types';

  interface Props {
    ctx: BootContext;
    mctx: AddModalContext;
  }

  let { ctx, mctx }: Props = $props();

  // Render-once by design: the browser mounts one modal per command from a
  // single boot context, so this intentionally captures the initial props.
  // svelte-ignore state_referenced_locally
  const modal = addArgsData(ctx, mctx);
</script>

<div
  class="addbutton-modal-container-args {modal.dark}"
  id="modal-container-{modal.id}"
  arg_modal_ID={modal.id}
>
  <div class="addbutton-modal-content-args {modal.dark}">
    <div class="addbutton-modal-header-args bold modal-container-{modal.id}">
      <h1 class="addbutton-modal-args"> {text('configure_your_button')}: {modal.buttonTitle}</h1>
      <div class="addbutton-modal-close-args">
        <CloseIcon cls="addbutton-args-config-modal" dark={modal.dark} />
      </div>
    </div>
    <div class="addbutton-modal-main-args">
      <div class="config-container {modal.dark}">
        <form class="args-form" arg_modal_ID={modal.id} novalidate>
          <ArgsBlock
            ctx={ctx}
            category={mctx.category}
            command={mctx.command}
            subId={mctx.subId}
            parentCommand={mctx.parentCommand}
            commandValue={mctx.commandValue}
            modalId={modal.id}
            idAttr="arg_modal_ID"
          />
          {#if modal.hasArgs}
            <div class="editorStyle-bar {modal.dark}"></div>
          {/if}
          <EditorStyle
            dark={modal.dark}
            id={modal.id}
            preview={modal.preview}
            defaultSize={modal.defaultSize}
            backgroundColor=""
            buttonName={modal.buttonName}
            nameValue=""
          />
          <div class="editorStyle-bar {modal.dark}" style="display: none;"></div>
          <div class="arg_container" arg_modal_ID={modal.id} style="display: none;">
            <label for="command_{modal.id}">Command (experimental):</label>
            <input
              class={modal.dark}
              type="text"
              name=""
              id="command_{modal.id}"
              value={modal.command}
              readonly
            />
          </div>
          <input
            type="submit"
            value={text('save')}
            id="{modal.id}_submit"
            class="createbutton_submit {modal.dark}"
            style="margin-top: 30px;"
          />
        </form>
      </div>
    </div>
  </div>
</div>
