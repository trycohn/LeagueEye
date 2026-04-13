import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FavoritePlayer, FrequentTeammate } from "../lib/types";

export function useFavorites() {
  const [favorites, setFavorites] = useState<FavoritePlayer[]>([]);
  const [suggestedTeammates, setSuggestedTeammates] = useState<FrequentTeammate[]>([]);
  const [loadingSuggested, setLoadingSuggested] = useState(false);

  const loadFavorites = useCallback(async () => {
    try {
      const favs = await invoke<FavoritePlayer[]>("get_favorites");
      setFavorites(favs);
    } catch (e) {
      console.error("get_favorites error:", e);
    }
  }, []);

  // Load favorites on mount
  useEffect(() => {
    loadFavorites();
  }, [loadFavorites]);

  const addFavorite = useCallback(
    async (
      puuid: string,
      gameName: string,
      tagLine: string,
      profileIconId: number,
      source: string = "manual"
    ) => {
      try {
        await invoke("add_favorite", {
          puuid,
          gameName,
          tagLine,
          profileIconId,
          source,
        });
        await loadFavorites();
      } catch (e) {
        console.error("add_favorite error:", e);
      }
    },
    [loadFavorites]
  );

  const removeFavorite = useCallback(
    async (puuid: string) => {
      try {
        await invoke("remove_favorite", { puuid });
        await loadFavorites();
      } catch (e) {
        console.error("remove_favorite error:", e);
      }
    },
    [loadFavorites]
  );

  const isFavorite = useCallback((puuid: string) => {
    return favorites.some((f) => f.puuid === puuid);
  }, [favorites]);

  const loadSuggestedTeammates = useCallback(async (puuid: string) => {
    setLoadingSuggested(true);
    try {
      const teammates = await invoke<FrequentTeammate[]>(
        "get_frequent_teammates",
        { puuid }
      );
      // Filter out already-favorited players
      const favPuuids = new Set(favorites.map((f) => f.puuid));
      setSuggestedTeammates(
        teammates.filter((t) => !favPuuids.has(t.puuid))
      );
    } catch (e) {
      console.error("get_frequent_teammates error:", e);
      setSuggestedTeammates([]);
    } finally {
      setLoadingSuggested(false);
    }
  }, [favorites]);

  return {
    favorites,
    suggestedTeammates,
    loadingSuggested,
    loadFavorites,
    addFavorite,
    removeFavorite,
    isFavorite,
    loadSuggestedTeammates,
  };
}
