import { describe, expect, it } from 'vitest';
import { svgInlineStyle } from './svg';

describe('svgInlineStyle', () => {
  it('emits px units on both dimensions', () => {
    // A unitless height is dropped by the CSS parser: the SVG then falls
    // back to its intrinsic height (16px Bootstrap icons rendered tiny).
    expect(svgInlineStyle(87, '')).toBe('style="width:87px; height:87px; "');
  });

  it('appends the fill declaration unchanged', () => {
    expect(svgInlineStyle(59, 'fill:#fff; color:#fff;')).toBe(
      'style="width:59px; height:59px; fill:#fff; color:#fff;"'
    );
  });
});
