import { describe, expect, it } from 'vitest';
import { bySize } from './sizes';

describe('size order', () => {
  it('puts letter sizes in chart order, then numbers, then the rest', () => {
    expect(['L', 'M', 'S', 'XL', 'XXL', 'XS'].sort(bySize)).toEqual(['XS', 'S', 'M', 'L', 'XL', 'XXL']);
    expect(['3XL', 'XL', '2XL', 'M'].sort(bySize)).toEqual(['M', 'XL', '2XL', '3XL']);
    expect(['32', '28', '30', 'Free', 'M'].sort(bySize)).toEqual(['M', '28', '30', '32', 'Free']);
    expect(['s', 'M', 'l'].sort(bySize)).toEqual(['s', 'M', 'l']);
  });
});
