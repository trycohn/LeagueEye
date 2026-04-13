import { Star, X, UserPlus, Users, Loader2 } from "lucide-react";
import { profileIconUrl } from "../lib/ddragon";
import type { FavoritePlayer, FrequentTeammate } from "../lib/types";

interface Props {
  favorites: FavoritePlayer[];
  suggestedTeammates: FrequentTeammate[];
  loadingSuggested: boolean;
  onPlayerClick: (gameName: string, tagLine: string) => void;
  onRemoveFavorite: (puuid: string) => void;
  onAddSuggested: (teammate: FrequentTeammate) => void;
}

export function FavoritesPanel({
  favorites,
  suggestedTeammates,
  loadingSuggested,
  onPlayerClick,
  onRemoveFavorite,
  onAddSuggested,
}: Props) {
  const hasFavorites = favorites.length > 0;
  const hasSuggested = suggestedTeammates.length > 0;

  if (!hasFavorites && !hasSuggested && !loadingSuggested) {
    return null;
  }

  return (
    <div className="flex flex-col gap-4 mb-6">
      {/* Favorites */}
      {hasFavorites && (
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <Star size={16} className="text-[#eab308]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">
              Избранные игроки
            </h2>
            <span className="text-xs text-[#64748b] ml-auto">
              {favorites.length}
            </span>
          </div>
          <div className="p-3 flex flex-wrap gap-2">
            {favorites.map((fav) => (
              <div
                key={fav.puuid}
                className="group flex items-center gap-2 px-3 py-2 bg-[#1e2130] border border-[#2a2d3a] rounded-sm hover:border-[#3b82f6]/50 transition-colors cursor-pointer"
                onClick={() => onPlayerClick(fav.gameName, fav.tagLine)}
              >
                <img
                  src={profileIconUrl(fav.profileIconId)}
                  alt={fav.gameName}
                  className="w-8 h-8 rounded-sm border border-[#2a2d3a]"
                />
                <div className="flex flex-col">
                  <span className="text-sm font-semibold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">
                    {fav.gameName}
                  </span>
                  <span className="text-xs text-[#64748b]">#{fav.tagLine}</span>
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemoveFavorite(fav.puuid);
                  }}
                  className="ml-1 p-0.5 rounded-sm text-[#64748b] hover:text-[#ef4444] opacity-0 group-hover:opacity-100 transition-all"
                  title="Убрать из избранного"
                >
                  <X size={14} />
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Suggested Teammates */}
      {(hasSuggested || loadingSuggested) && (
        <div className="bg-[#1a1d28] border border-[#2a2d3a] rounded-sm">
          <div className="px-4 py-3 border-b border-[#2a2d3a] flex items-center gap-2">
            <Users size={16} className="text-[#3b82f6]" />
            <h2 className="text-sm font-bold text-[#e2e8f0] uppercase tracking-wider">
              Частые тиммейты
            </h2>
          </div>
          {loadingSuggested ? (
            <div className="p-4 flex items-center justify-center gap-2 text-[#64748b]">
              <Loader2 size={16} className="animate-spin" />
              <span className="text-sm">Поиск тиммейтов...</span>
            </div>
          ) : (
            <div className="p-0">
              <table className="w-full text-left text-sm">
                <thead className="bg-[#1e2130] text-[#64748b] text-xs uppercase">
                  <tr>
                    <th className="px-4 py-2 font-semibold">Игрок</th>
                    <th className="px-4 py-2 font-semibold text-right">Совместных игр</th>
                    <th className="px-4 py-2 font-semibold text-right">Винрейт</th>
                    <th className="px-4 py-2 font-semibold text-right w-10"></th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-[#2a2d3a]">
                  {suggestedTeammates.map((t) => (
                    <tr
                      key={t.puuid}
                      className="hover:bg-[#252838] transition-colors cursor-pointer group"
                      onClick={() => onPlayerClick(t.gameName, t.tagLine)}
                    >
                      <td className="px-4 py-3">
                        <span className="font-bold text-[#e2e8f0] group-hover:text-[#3b82f6] transition-colors">
                          {t.gameName}
                        </span>
                        <span className="text-[#64748b] ml-1">#{t.tagLine}</span>
                      </td>
                      <td className="px-4 py-3 text-right font-medium text-[#94a3b8]">
                        {t.gamesTogether}
                      </td>
                      <td className={`px-4 py-3 text-right font-medium ${
                        t.winrate >= 50 ? "text-[#22c55e]" : "text-[#ef4444]"
                      }`}>
                        {t.winrate.toFixed(1)}%
                      </td>
                      <td className="px-4 py-3 text-right">
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            onAddSuggested(t);
                          }}
                          className="p-1 rounded-sm text-[#64748b] hover:text-[#eab308] transition-colors"
                          title="Добавить в избранное"
                        >
                          <UserPlus size={16} />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
