<script lang="ts">
  import type { Snippet } from 'svelte';
  import CloseIcon from '../CloseIcon.svelte';
  import { stringAttrs } from '../string-attrs';

  /**
   * Reusable accessible modal shell for all four modal kinds (config,
   * add-button browser, add-args, edit-button). One place for the
   * backdrop/content/header/close chrome and the dialog semantics; each
   * caller keeps its own wiring-critical hook classes, ids, and modal-id
   * attributes, passed through as props.
   */

  interface Props {
    /** Outer hook classes, e.g. `modal-container dark-theme`. */
    containerClass: string;
    containerId?: string | undefined;
    /** Modal-id attributes (`edit_modal_ID`, `arg_modal_ID`). */
    containerAttrs?: Record<string, string> | undefined;
    /** Content hook classes, e.g. `modal-content dark-theme`. */
    contentClass: string;
    contentId?: string | undefined;
    /** Header hook classes (keeps per-modal extras). */
    headerClass: string;
    /** Heading hook class (`config-modal`, ...). */
    titleClass: string;
    /** Heading text (pre-composed, already translated). */
    title: string;
    /** Mono badge after the title (`/key`, `v1.8.7`). */
    badge?: string | undefined;
    /** Count pill after the title (browser only). */
    count?: number | undefined;
    /** Close hook class (`modal-close`, ... — global wiring target). */
    closeClass: string;
    /** CloseIcon `cls` prop (per-modal hook). */
    closeIconClass: string;
    dark: string;
    /** Unique heading id; the dialog is labelled by it. */
    labelledBy: string;
    children?: Snippet | undefined;
  }

  let {
    containerClass,
    containerId,
    containerAttrs,
    contentClass,
    contentId,
    headerClass,
    titleClass,
    title,
    badge,
    count,
    closeClass,
    closeIconClass,
    dark,
    labelledBy,
    children,
  }: Props = $props();
</script>

<div class={containerClass} id={containerId} use:stringAttrs={containerAttrs ?? {}}>
  <div
    class={contentClass}
    id={contentId}
    role="dialog"
    aria-modal="true"
    aria-labelledby={labelledBy}
    tabindex="-1"
    data-wd2-dialog={labelledBy}
  >
    <div class={headerClass}>
      <h1 class={titleClass} id={labelledBy}>
        {title}{#if badge !== undefined && badge !== ''}<span class="wd2-cmd">{badge}</span>{/if}{#if count !== undefined}<span class="wd2-count">{count}</span>{/if}
      </h1>
      <div class={closeClass}>
        <CloseIcon cls={closeIconClass} dark={dark} />
      </div>
    </div>
    {#if children !== undefined}{@render children()}{/if}
  </div>
</div>
