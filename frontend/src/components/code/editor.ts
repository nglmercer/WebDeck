import { EditorState, Compartment } from '@codemirror/state';
import {
  EditorView,
  keymap,
  lineNumbers,
  drawSelection,
  highlightActiveLine,
} from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import {
  syntaxHighlighting,
  HighlightStyle,
  bracketMatching,
  indentOnInput,
} from '@codemirror/language';
import { javascript } from '@codemirror/lang-javascript';
import { json } from '@codemirror/lang-json';
import { tags } from '@lezer/highlight';
export function mountEditor(
  parent: HTMLElement,
  doc: string,
  language: string,
  label: string,
  onchange: (text: string) => void,
  compact: boolean,
) {
  const mode = new Compartment();
  const languageFor = (name: string) =>
    name === 'javascript' ? javascript() : name === 'json' ? json() : [];
  const highlight = HighlightStyle.define([
    { tag: tags.keyword, color: '#c89dff' },
    { tag: [tags.string, tags.special(tags.string)], color: '#f5b886' },
    { tag: [tags.number, tags.bool, tags.null], color: '#bca9ff' },
    { tag: tags.comment, color: '#8995b1' },
    { tag: [tags.propertyName, tags.variableName], color: '#a2d9f2' },
  ]);
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc,
      extensions: [
        ...(compact ? [] : [lineNumbers(), highlightActiveLine()]),
        history(),
        drawSelection(),
        bracketMatching(),
        indentOnInput(),
        syntaxHighlighting(highlight),
        mode.of(languageFor(language)),
        EditorView.lineWrapping,
        keymap.of([...defaultKeymap, ...historyKeymap]),
        EditorView.contentAttributes.of({
          'aria-label': label,
          spellcheck: 'false',
          'aria-multiline': 'true',
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) onchange(update.state.doc.toString());
        }),
        EditorView.theme(
          {
            '&': { fontSize: '.85rem', color: 'var(--text)', backgroundColor: 'var(--canvas)' },
            '.cm-scroller': {
              fontFamily: 'ui-monospace, SFMono-Regular, Consolas, monospace',
              lineHeight: '1.65',
              overflow: 'auto',
              minHeight: compact ? '48px' : '180px',
              maxHeight: compact ? '120px' : '380px',
            },
            '.cm-content': { padding: '10px 0', caretColor: 'var(--accent)' },
            '.cm-line': { padding: '0 12px' },
            '.cm-gutters': {
              backgroundColor: 'var(--canvas)',
              color: 'var(--text-muted)',
              borderRight: '1px solid var(--border-subtle)',
            },
            '.cm-activeLine': { backgroundColor: '#ffffff06' },
            '&.cm-focused': { outline: 'none' },
            '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
              backgroundColor: '#7562df66',
            },
          },
          { dark: true },
        ),
      ],
    }),
  });
  return {
    view,
    setValue(text: string) {
      if (view.state.doc.toString() !== text)
        view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
    },
    setLanguage(name: string) {
      view.dispatch({ effects: mode.reconfigure(languageFor(name)) });
    },
    destroy() {
      view.destroy();
    },
  };
}
