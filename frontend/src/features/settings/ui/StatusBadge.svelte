<script lang="ts">
  let { label, tone = 'neutral' }: { label: string; tone?: 'neutral' | 'success' | 'error' } =
    $props();
</script>

<span class="status-badge" data-tone={tone}><span aria-hidden="true"></span>{label}</span>

<style>
  .status-badge {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    color: var(--text-muted);
    font-size: 0.75rem;
    padding: 5px 9px;
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
  }
  .status-badge > span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: currentColor;
  }
  [data-tone='success'] {
    color: #adc3b4;
  }
  [data-tone='error'] {
    color: #d2aaaa;
  }
</style>
