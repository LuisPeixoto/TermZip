import { describe, expect, it } from "vitest";
import { ANSI_LIGHT } from "../src/lib/terminal/xtermTheme";

const LIGHT_BACKGROUND = "#fafafa";

function luminance(hex: string): number {
  const channel = (offset: number) => {
    const value = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

describe("light terminal palette", () => {
  it("keeps white, bright black and bright white readable on the light background", () => {
    for (const index of [7, 8, 15]) {
      expect(contrast(ANSI_LIGHT[index], LIGHT_BACKGROUND), `color ${index}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("keeps every color distinguishable from the background", () => {
    for (const color of ANSI_LIGHT) {
      expect(contrast(color, LIGHT_BACKGROUND)).toBeGreaterThanOrEqual(3);
    }
  });
});
