import type { JsonObject } from '../framework/types';
import { pageState } from './state';

// Usage polling (index.jinja window-load block). Shared tile updater is used
// by both the interval and the click-to-refresh path; the contained-eval fix
// applies to both (see docs/MIGRATION_RUST.md deviations).
//
// Upstream uses direct `eval(path)`; these paths are data-built dotted
// lookups whose only free variable is the `usage_dict` parameter, so a
// `Function` constructor scoped to that parameter is exactly equivalent
// (same globals, same member-access semantics) without tripping the
// bundler's eval warning. It is also evaluated once instead of up to
// three times; member access has no side effects, so this is 1:1.
function evalUsagePath(usage_dict: JsonObject, path: string): string {
  const fn = new Function('usage_dict', `return (${path});`) as (
    usage_dict: JsonObject
  ) => string;
  return fn(usage_dict);
}

export function updateUsageTiles(usage_dict: JsonObject): void {
  const divElements = document.querySelectorAll('.usage-value');
  divElements.forEach((divElement) => {
    const otherClass = divElement.classList[1] as string | undefined;
    if (otherClass !== undefined && otherClass.includes('.')) {
      // One bad tile path must not abort the whole update loop.
      try {
        const evaluated = evalUsagePath(usage_dict, otherClass);
        if (evaluated !== '-') {
          let newValue: string;
          if (otherClass.includes('percent')) {
            newValue = evaluated + '%';
          } else if (otherClass.includes('_gb')) {
            newValue = evaluated + ' gb';
          } else if (otherClass.includes('_mb')) {
            newValue = evaluated + ' mb';
          } else if (otherClass.includes('bytes')) {
            newValue = evaluated + ' bytes';
          } else {
            newValue = evaluated;
          }
          if (!newValue.includes('undefined')) {
            divElement.textContent = newValue;
          }
        }
      } catch {
        return;
      }
    }
  });
}

export function pollUsageOnce(): void {
  const forms = document.querySelectorAll('form');
  const messages: Array<{ form: HTMLFormElement; message: string }> = [];
  forms.forEach((form) => {
    const messageElement = form.querySelector('.message') as HTMLInputElement | null;
    if (messageElement) {
      messages.push({ form, message: messageElement.value });
    }
  });

  try {
    fetch('/usage', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ messages }),
    })
      .then((response) => response.json())
      .then((usage_dict: JsonObject) => {
        if (pageState.disconnectCount > 0) {
          const loadingScreen = document.getElementById('loading-screen');
          loadingScreen?.classList.add('hidden');
          pageState.disconnectCount = 0;
        }
        // NOTE: upstream repeats this update once per form (identical
        // result); a single pass is equivalent.
        updateUsageTiles(usage_dict);
      })
      .catch(function () {
        pageState.disconnectCount++;
        if (pageState.disconnectCount > 3) {
          const loadingScreen = document.getElementById('loading-screen');
          loadingScreen?.classList.remove('hidden');
        }
      });
  } catch (e) {
    console.log((e as Error).message);
  }
}

/** Starts the usage poll after first paint (window-load equivalent). */
export function startUsageLoop(reloadMs: number): void {
  setInterval(function () {
    pollUsageOnce();
  }, reloadMs);
}
