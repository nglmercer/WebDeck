// Port of static/js/themes-setting.js. Runs after render.
import { contains, q, byId } from '../query';

function getThemesArray(): string[] {
  const themesArrayString = String(byId<HTMLInputElement>('choose-themes-handler').val() ?? '[]').replace(
    /'/g,
    '"'
  );
  return JSON.parse(themesArrayString) as string[];
}

function updateThemesInputValue(themesArray: string[]): void {
  const modifiedArray = themesArray.toString().replace(/,/g, "','");
  // NOTE: the *attribute* (default value), not the property — .attr(), not .val().
  byId('choose-themes-handler').attr('value', `['${modifiedArray}']`);
}

function swapElements(list: string[], firstElement: string, secondElement: string): string[] | null {
  const firstIndex = list.indexOf(firstElement);
  const secondIndex = list.indexOf(secondElement);

  if (firstIndex !== -1 && secondIndex !== -1) {
    const temp = list[firstIndex] as string;
    list[firstIndex] = list[secondIndex] as string;
    list[secondIndex] = temp;
    return list;
  } else {
    console.log('One or both elements are not found in the list.');
    return null;
  }
}

function swapDomElements(element1: Element, element2: Element): void {
  const parent = q(element1).parent().get(0) ?? null;
  if (!parent) return;
  // Same live-index dance as the original: indices are captured first,
  // the sibling list is re-read between the two moves, and a missing
  // index appends (like insertBefore with null).
  const kids = (): Element[] => q(parent).children().toArray();
  const first = kids();
  const index1 = first.indexOf(element1);
  const index2 = first.indexOf(element2);
  const target1 = kids()[index2];
  if (target1) q(element1).insertBefore(target1);
  else q(element1).appendTo(parent);
  const target2 = kids()[index1];
  if (target2) q(element2).insertBefore(target2);
  else q(element2).appendTo(parent);

  const themesArray = getThemesArray();
  swapElements(themesArray, q(element1).attr('filename') ?? '', q(element2).attr('filename') ?? '');
  updateThemesInputValue(themesArray);
}

function handleArrowClick(this: Element, event: Event): void {
  const target = event.target as Element;
  const themeContainer = q(this).closest('.theme-container').get(0) ?? null;
  const parentContainer = themeContainer ? q(themeContainer).parent().get(0) ?? null : null;
  if (!themeContainer || !parentContainer) return;

  if (q(target).hasClass('arrow-up-hitbox')) {
    const prevSibling = q(themeContainer).prev().get(0) ?? null;
    if (prevSibling !== null && q(prevSibling).attr('defaulttheme') === undefined) {
      swapDomElements(themeContainer, prevSibling);
    }
  } else if (q(target).hasClass('arrow-down-hitbox')) {
    const nextSibling = q(themeContainer).next().get(0) ?? null;
    if (nextSibling !== null && q(nextSibling).attr('defaulttheme') === undefined) {
      swapDomElements(nextSibling, themeContainer);
    }
  }

  const firstTheme = q(parentContainer).children().first().get(0) ?? null;
  const lastTheme = q(parentContainer).children().last().get(0) ?? null;
  q(themeContainer)
    .find('.arrow-up-hitbox, .arrow-down-hitbox')
    .toArray()
    .forEach((arrow) => {
      if (
        (themeContainer === firstTheme || q(firstTheme).attr('defaulttheme') !== undefined) &&
        q(arrow).hasClass('arrow-up-hitbox')
      ) {
        q(arrow).addClass('disabled');
      } else {
        q(arrow).removeClass('disabled');
      }
      if (
        (themeContainer === lastTheme || q(lastTheme).attr('defaulttheme') !== undefined) &&
        q(arrow).hasClass('arrow-down-hitbox')
      ) {
        q(arrow).addClass('disabled');
      } else {
        q(arrow).removeClass('disabled');
      }
    });
}

export function initThemesSetting(): void {
  // NOTE: panel visibility is owned by the config tab state — no
  // back-button navigation. Only enable/disable/reorder wiring below.
  q('.theme-container')
    .toArray()
    .forEach((container) => {
      const disableArrow = q(container).find('.disable-theme').get(0) ?? null;
      const moveArrow = disableArrow ?? q(container).find('.enable-theme').get(0) ?? null;
      const upDownArrows = q(container).find('.arrows-container').get(0) ?? null;

      if (moveArrow !== null) {
        q(container).hover(
          function () {
            q(moveArrow).removeClass('invisible');
          },
          function () {
            q(moveArrow).addClass('invisible');
          }
        );
      }

      if (upDownArrows !== null) {
        q(container).hover(
          function () {
            const parentDiv = byId('disabled-themes').get(0) ?? null;
            // `.arrows-container` is never `#disabled-themes` itself, so
            // qdom's contains() matches the native call here.
            const isChild = parentDiv ? contains(parentDiv, upDownArrows) : false;
            if (!isChild) {
              q(upDownArrows).removeClass('invisible');
            }
          },
          function () {
            q(upDownArrows).addClass('invisible');
          }
        );
      }
    });

  q('.disable-theme-hitbox, .enable-theme-hitbox')
    .toArray()
    .forEach((hitbox) => {
      q(hitbox).on('click', function (this: Element) {
        const themeContainer = q(this).closest('.theme-container').get(0) ?? null;
        const disableArrow = themeContainer
          ? q(themeContainer).find('.disable-theme').get(0) ?? null
          : null;
        const arrow = disableArrow ?? (themeContainer ? q(themeContainer).find('.enable-theme').get(0) ?? null : null);
        const upDownArrows = themeContainer
          ? q(themeContainer).find('.arrows-container').get(0) ?? null
          : null;

        console.log(arrow);
        console.log('Clicked:', q(this).hasClass('disable-theme-hitbox') ? 'Disable Theme' : 'Enable Theme');
        console.log('Parent .theme-container:', themeContainer);

        const themePath = themeContainer ? q(themeContainer).attr('filename') ?? '' : '';
        const themesArray = getThemesArray();
        let newElement: string | undefined;

        console.log(themesArray);

        for (let i = 0; i < themesArray.length; i++) {
          if ((themesArray[i] ?? '').replace('//', '') === themePath.replace('//', '')) {
            if (!(themesArray[i] ?? '').startsWith('//')) {
              newElement = '//' + (themesArray[i] ?? '');
              themesArray.splice(i, 1);
              themesArray.unshift(newElement);
              const disabled = byId('disabled-themes').get(0) ?? null;
              if (disabled && themeContainer) q(disabled).prepend(themeContainer);

              q(this).removeClass('disable-theme-hitbox');
              q(this).addClass('enable-theme-hitbox');

              q(arrow).removeClass('disable-theme');
              q(arrow).addClass('enable-theme');

              q(upDownArrows).addClass('invisible');
            } else {
              newElement = (themesArray[i] ?? '').replace('//', '');
              themesArray.splice(i, 1);
              themesArray.unshift(newElement);
              const enabled = byId('enabled-themes').get(0) ?? null;
              if (enabled && themeContainer) q(enabled).prepend(themeContainer);

              q(this).removeClass('enable-theme-hitbox');
              q(this).addClass('disable-theme-hitbox');

              q(arrow).removeClass('enable-theme');
              q(arrow).addClass('disable-theme');

              q(upDownArrows).removeClass('invisible');
            }
          }
        }
        void newElement;

        console.log(themesArray);

        updateThemesInputValue(themesArray);
      });
    });

  q('.arrow-up-hitbox, .arrow-down-hitbox')
    .toArray()
    .forEach((arrow) => {
      q(arrow).on('click', handleArrowClick);
    });
}

console.log('themes-setting.js loaded');
