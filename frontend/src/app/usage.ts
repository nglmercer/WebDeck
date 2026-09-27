import type { JsonObject } from '../framework/types';
import { q, byId } from '../query';
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
  q('.usage-value')
    .toArray()
    .forEach((divElement) => {
      // Equivalent to `classList[1]` (second class token or undefined).
      const otherClass = q(divElement)
        .attr('class')
        ?.split(/\s+/)
        .filter((token) => token !== '')[1];
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
              q(divElement).text(newValue);
            }
          }
        } catch {
          return;
        }
      }
    });
}

export function pollUsageOnce(): void {
  const messages: Array<{ form: HTMLFormElement; message: string }> = [];
  q('form')
    .toArray()
    .forEach((form) => {
      const messageElement = q(form).find('.message').get(0) ?? null;
      if (messageElement) {
        messages.push({ form, message: String(q(messageElement).val() ?? '') });
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
          byId('loading-screen').addClass('hidden');
          pageState.disconnectCount = 0;
        }
        // NOTE: upstream repeats this update once per form (identical
        // result); a single pass is equivalent.
        updateUsageTiles(usage_dict);
      })
      .catch(function () {
        pageState.disconnectCount++;
        if (pageState.disconnectCount > 3) {
          byId('loading-screen').removeClass('hidden');
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
