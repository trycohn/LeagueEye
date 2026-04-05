import { useState, useEffect } from "react";

const DDRAGON_VERSION = "16.7.1";
const URL = `https://ddragon.leagueoflegends.com/cdn/${DDRAGON_VERSION}/data/en_US/champion.json`;

let cachedMap: Record<number, string> | null = null;

export function useChampionNames(): Record<number, string> {
  const [map, setMap] = useState<Record<number, string>>(cachedMap || {});

  useEffect(() => {
    if (cachedMap) return;

    fetch(URL)
      .then((r) => r.json())
      .then((data) => {
        const result: Record<number, string> = {};
        for (const [, champ] of Object.entries(data.data) as [
          string,
          { key: string; id: string },
        ][]) {
          result[Number(champ.key)] = champ.id;
        }
        cachedMap = result;
        setMap(result);
      })
      .catch(() => {});
  }, []);

  return map;
}
