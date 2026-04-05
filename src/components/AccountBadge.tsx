import type { DetectedAccount } from "../lib/types";

const DD_VERSION = "15.6.1";

function iconUrl(iconId: number) {
  return `https://ddragon.leagueoflegends.com/cdn/${DD_VERSION}/img/profileicon/${iconId}.png`;
}

interface Props {
  account: DetectedAccount;
  clientOnline: boolean;
  onClick: () => void;
}

export function AccountBadge({ account, clientOnline, onClick }: Props) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-bg-secondary transition-colors shrink-0 group"
      title="Открыть мой профиль"
    >
      <div className="relative">
        <img
          src={iconUrl(account.profileIconId)}
          alt="avatar"
          className="w-8 h-8 rounded-full border border-border group-hover:border-accent transition-colors"
          onError={(e) => {
            (e.target as HTMLImageElement).src =
              `https://ddragon.leagueoflegends.com/cdn/${DD_VERSION}/img/profileicon/1.png`;
          }}
        />
        <span
          className={`absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-bg-primary ${
            clientOnline ? "bg-green-500" : "bg-gray-500"
          }`}
        />
      </div>
      <div className="flex flex-col items-start leading-tight">
        <span className="text-sm font-medium text-text-primary group-hover:text-accent transition-colors">
          {account.gameName}
        </span>
        <span className="text-xs text-text-muted">#{account.tagLine}</span>
      </div>
    </button>
  );
}
