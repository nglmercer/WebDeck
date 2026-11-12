<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '../../../components/Icon.svelte';
  let {
    title,
    icon,
    children,
    id,
    description,
    actions,
  }: {
    title: string;
    icon?: string;
    children: Snippet;
    id?: string;
    description?: string;
    actions?: Snippet;
  } = $props();
</script>

<section class="settings-card" {id}>
  <div class="settings-card-heading">
    {#if icon}<Icon name={icon} size={20} />{/if}
    <h3>{title}</h3>
    {@render actions?.()}
  </div>
  {#if description}<p class="settings-card-description">{description}</p>{/if}
  {@render children()}
</section>

<style>
  .settings-card {
    min-width: 0;
    margin: 0;
    padding: 20px;
    background: var(--surface);
    border: 1px solid var(--border-subtle);
    border-radius: 10px;
  }
  .settings-card-heading {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 16px;
  }
  h3 {
    flex: 1;
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
  }
  .settings-card-description {
    margin: -4px 0 16px;
  }
  @media (max-width: 600px) {
    .settings-card {
      padding: 16px;
    }
  }
</style>
