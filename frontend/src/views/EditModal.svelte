<script lang="ts">
  import EditorStyle from '../components/EditorStyle.svelte';
  import ModalShell from '../components/studio/ModalShell.svelte';
  import StudioTabs from '../components/studio/StudioTabs.svelte';
  import StudioField from '../components/studio/StudioField.svelte';
  import { tx } from '../components/studio/labels';
  import { hide_editbutton_modal } from '../app/modals';
  import { text } from '../framework/i18n';
  import { asString, type BootContext, type JsonObject } from '../framework/types';
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

  const cmdBadge = modal.args !== null ? asString(modal.args.commandValue['command']) : '';
  const tabs = modal.args !== null
    ? [
        { id: 'args', label: tx('studio_tab_params', 'Parameters') },
        { id: 'look', label: tx('studio_tab_appearance', 'Appearance') },
      ]
    : [{ id: 'look', label: tx('studio_tab_appearance', 'Appearance') }];
  let selectedTab = $state('args');
  const activeTab = $derived(modal.args !== null ? selectedTab : 'look');
</script>

<ModalShell
  containerClass="editbutton-modal-container {modal.dark}"
  containerId="edit-modal-container-{modal.modalId}"
  containerAttrs={{ edit_modal_ID: modal.modalId }}
  contentClass="editbutton-modal-content {modal.dark}"
  headerClass="editbutton-modal-header bold edit-modal-container-{modal.modalId}"
  titleClass="editbutton-modal"
  title={text('configure_your_button')}
  badge={cmdBadge}
  closeClass="editbutton-modal-close"
  closeIconClass="editbutton-config-modal"
  dark={modal.dark}
  labelledBy="edit-{modal.modalId}-title"
>
  <div class="editbutton-modal-main">
    <div class="config-container {modal.dark}">
      <form class="args-form" edit_modal_ID={modal.modalId} novalidate>
        <div class="wd2-split">
          <aside class="wd2-preview" aria-label={tx('studio_preview', 'Preview')}>
            <h2 class="wd2-pane-title">{tx('studio_preview', 'Preview')}</h2>
            <EditorStyle
              dark={modal.dark}
              id={modal.modalId}
              preview={modal.preview}
              defaultSize={modal.defaultSize}
              backgroundColor={modal.backgroundColor}
              buttonName={modal.buttonName}
              nameValue={modal.nameValue}
              mode="preview"
            />
            <div class="wd2-meta">
              <div class="mrow"><span>{tx('studio_meta_key', 'Key')}</span><b id="meta-key_{modal.modalId}">—</b></div>
              <div class="mrow"><span>{tx('studio_meta_size', 'Size')}</span><b id="meta-size_{modal.modalId}">—</b></div>
              <div class="mrow"><span>{tx('studio_meta_bg', 'Background')}</span><b id="meta-color_{modal.modalId}">—</b></div>
            </div>
          </aside>
          <div class="wd2-formcol">
            <StudioTabs
              tabs={tabs}
              selected={activeTab}
              onSelect={(id) => (selectedTab = id)}
              idPrefix="edit-{modal.modalId}"
            />
            <div class="wd2-panes">
              {#if modal.args !== null}
                <div
                  class="wd2-pane"
                  role="tabpanel"
                  id="edit-{modal.modalId}-pane-args"
                  aria-labelledby="edit-{modal.modalId}-tab-args"
                  hidden={activeTab !== 'args'}
                >
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
                </div>
              {/if}
              <div
                class="wd2-pane"
                role="tabpanel"
                id="edit-{modal.modalId}-pane-look"
                aria-labelledby="edit-{modal.modalId}-tab-look"
                hidden={activeTab !== 'look'}
              >
                <EditorStyle
                  dark={modal.dark}
                  id={modal.modalId}
                  preview={modal.preview}
                  defaultSize={modal.defaultSize}
                  backgroundColor={modal.backgroundColor}
                  buttonName={modal.buttonName}
                  nameValue={modal.nameValue}
                  mode="controls"
                />
                {#if modal.showDevbox}
                  <div class="editorStyle-bar {modal.dark}"></div>
                  <div class="arg_container" edit_modal_ID={modal.modalId}>
                    <StudioField label={text('edit_command')} labelFor="command_{modal.modalId}">
                      <input
                        class={modal.dark}
                        type="text"
                        name=""
                        id="command_{modal.modalId}"
                        value={modal.devboxValue}
                      />
                    </StudioField>
                  </div>
                {/if}
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
              onclick={() => hide_editbutton_modal(modal.modalId)}
            >
              {tx('cancel', 'Cancel')}
            </button>
            <input
              type="submit"
              id="{modal.modalId}_submit"
              class="modal-button save-config {modal.dark}"
              value={text('save')}
            />
          </div>
        </footer>
      </form>
    </div>
  </div>
</ModalShell>
