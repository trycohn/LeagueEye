import { positionIconUrl } from "../lib/ddragon";

/** Настоящие иконки ролей из CommunityDragon (SVG). */
export function RoleIcon({ role, size = 12 }: { role: string; size?: number }) {
  return (
    <img
      src={positionIconUrl(role)}
      alt={role}
      width={size}
      height={size}
      className="shrink-0 opacity-50"
      style={{ filter: "brightness(0) invert(1)" }}
      onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
    />
  );
}
