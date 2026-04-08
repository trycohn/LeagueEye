/** Варианты компактности оверлея золота — выбор через ?layout= в gold-overlay.html */
export type GoldOverlayLayout = "classic" | "compact" | "single" | "micro";

const LAYOUTS: GoldOverlayLayout[] = ["classic", "compact", "single", "micro"];

export function parseGoldOverlayLayout(search: string): GoldOverlayLayout {
  const q = new URLSearchParams(search).get("layout");
  if (q && LAYOUTS.includes(q as GoldOverlayLayout)) {
    return q as GoldOverlayLayout;
  }
  return "classic";
}

export function goldOverlayWidth(layout: GoldOverlayLayout): number {
  switch (layout) {
    case "classic":
      return 280;
    case "compact":
      return 252;
    case "single":
      return 244;
    case "micro":
      return 228;
    default:
      return 252;
  }
}
