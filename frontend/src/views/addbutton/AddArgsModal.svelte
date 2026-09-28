<script lang="ts">
  import EditorStyle from '../../components/EditorStyle.svelte';
  import ModalShell from '../../components/studio/ModalShell.svelte';
  import StudioTabs from '../../components/studio/StudioTabs.svelte';
  import { tx } from '../../components/studio/labels';
  import { hide_addbutton_args_modal } from '../../app/modals';
  import { text } from '../../framework/i18n';
  import { asString, type BootContext } from '../../framework/types';
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
  // svelte-ignore state_referenced_locally
  const cmdBadge = asString(mctx.commandValue['command']);

  const tabs = modal.hasArgs
    ? [
        { id: 'args', label: tx('studio_tab_params', 'Parameters') },
        { id: 'look', label: tx('studio_tab_appearance', 'Appearance') },
      ]
    : [{ id: 'look', label: tx('studio_tab_appearance', 'Appearance') }];
  let selectedTab = $state('args');
  const activeTab = $derived(modal.hasArgs ? selectedTab : 'look');
</script>

<ModalShell
  containerClass="addbutton-modal-container-args {modal.dark}"
  containerId="modal-container-{modal.id}"
  containerAttrs={{ arg_modal_ID: modal.id }}
  contentClass="addbutton-modal-content-args {modal.dark}"
  headerClass="addbutton-modal-header-args bold modal-container-{modal.id}"
  titleClass="addbutton-modal-args"
  title="{text('configure_your_button')}: {modal.buttonTitle}"
  badge={cmdBadge}
  closeClass="addbutton-modal-close-args"
  closeIconClass="addbutton-args-config-modal"
  dark={modal.dark}
  labelledBy="add-{modal.id}-title"
>
  <div class="addbutton-modal-main-args">
    <div class="config-container {modal.dark}">
      <form class="args-form" arg_modal_ID={modal.id} novalidate>
        <div class="wd2-split">
          <aside class="wd2-preview" aria-label={tx('studio_preview', 'Preview')}>
            <h2 class="wd2-pane-title">{tx('studio_preview', 'Preview')}</h2>
            <EditorStyle
              dark={modal.dark}
              id={modal.id}
              preview={modal.preview}
              defaultSize={modal.defaultSize}
              backgroundColor=""
              buttonName={modal.buttonName}
              nameValue=""
              mode="preview"
            />
            <div class="wd2-meta">
              <div class="mrow"><span>{tx('studio_meta_key', 'Key')}</span><b id="meta-key_{modal.id}">—</b></div>
              <div class="mrow"><span>{tx('studio_meta_size', 'Size')}</span><b id="meta-size_{modal.id}">—</b></div>
              <div class="mrow"><span>{tx('studio_meta_bg', 'Background')}</span><b id="meta-color_{modal.id}">—</b></div>
            </div>
          </aside>
          <div class="wd2-formcol">
            <StudioTabs
              tabs={tabs}
              selected={activeTab}
              onSelect={(id) => (selectedTab = id)}
              idPrefix="add-{modal.id}"
            />
            <div class="wd2-panes">
              {#if modal.hasArgs}
                <div
                  class="wd2-pane"
                  role="tabpanel"
                  id="add-{modal.id}-pane-args"
                  aria-labelledby="add-{modal.id}-tab-args"
                  hidden={activeTab !== 'args'}
                >
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
                </div>
              {/if}
              <div
                class="wd2-pane"
                role="tabpanel"
                id="add-{modal.id}-pane-look"
                aria-labelledby="add-{modal.id}-tab-look"
                hidden={activeTab !== 'look'}
              >
                <EditorStyle
                  dark={modal.dark}
                  id={modal.id}
                  preview={modal.preview}
                  defaultSize={modal.defaultSize}
                  backgroundColor=""
                  buttonName={modal.buttonName}
                  nameValue=""
                  mode="controls"
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
              </div>
            </div>
          </div>
        </div>
        <footer class="wd2-foot">
          <span class="wd2-hint">{tx('studio_live_hint', 'Changes preview live')}</span>
          <div class="wd2-actions">
            <button
              type="button"
              class="wd2-btn ghost"
              onclick={() => hide_addbutton_args_modal()}
            >
              {tx('cancel', 'Cancel')}
            </button>
            <input
              type="submit"
              value={text('save')}
              id="{modal.id}_submit"
              data-testid="add-args-save"
              class="createbutton_submit {modal.dark}"
            />
          </div>
        </footer>
      </form>
    </div>
  </div>
</ModalShell>
