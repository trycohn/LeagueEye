import type { GoldComparisonData } from "./types";

/** Тестовые данные для скриншотов оверлея в браузере (?mock=1, только dev). */
export const GOLD_OVERLAY_MOCK_DATA: GoldComparisonData = {
  gameTime: 912,
  lanes: [
    {
      role: "TOP",
      allyChampionName: "Garen",
      allyGold: 7200,
      enemyChampionName: "Darius",
      enemyGold: 6800,
      goldDiff: 400,
    },
    {
      role: "JUNGLE",
      allyChampionName: "LeeSin",
      allyGold: 6100,
      enemyChampionName: "Graves",
      enemyGold: 6550,
      goldDiff: -450,
    },
    {
      role: "MIDDLE",
      allyChampionName: "Ahri",
      allyGold: 7800,
      enemyChampionName: "Syndra",
      enemyGold: 7200,
      goldDiff: 600,
    },
    {
      role: "BOTTOM",
      allyChampionName: "Caitlyn",
      allyGold: 7500,
      enemyChampionName: "Jinx",
      enemyGold: 7600,
      goldDiff: -100,
    },
    {
      role: "UTILITY",
      allyChampionName: "Thresh",
      allyGold: 4200,
      enemyChampionName: "Lulu",
      enemyGold: 4100,
      goldDiff: 100,
    },
  ],
};
