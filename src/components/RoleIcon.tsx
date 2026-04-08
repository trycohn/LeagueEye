/** Мини-иконки ролей в стиле LoL (упрощённые inline SVG). */
export function RoleIcon({ role, size = 12 }: { role: string; size?: number }) {
  const s = size;
  const paths: Record<string, React.ReactNode> = {
    TOP: (
      <svg width={s} height={s} viewBox="0 0 16 16" fill="none">
        <path d="M3 13 L3 3 L13 3" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M3 3 L13 13" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"/>
      </svg>
    ),
    JUNGLE: (
      <svg width={s} height={s} viewBox="0 0 16 16" fill="currentColor">
        <path d="M8 1 C5 1 2 4 2 7 C2 10 4 12 6 13 L6 15 L10 15 L10 13 C12 12 14 10 14 7 C14 4 11 1 8 1Z" opacity="0.9"/>
        <rect x="6" y="13" width="4" height="1.5" rx="0.5"/>
      </svg>
    ),
    MIDDLE: (
      <svg width={s} height={s} viewBox="0 0 16 16" fill="none">
        <path d="M2 14 L14 2" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"/>
        <circle cx="8" cy="8" r="2.5" fill="currentColor"/>
      </svg>
    ),
    BOTTOM: (
      <svg width={s} height={s} viewBox="0 0 16 16" fill="none">
        <path d="M3 3 L13 3 L13 13" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"/>
        <path d="M13 13 L3 3" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"/>
      </svg>
    ),
    UTILITY: (
      <svg width={s} height={s} viewBox="0 0 16 16" fill="currentColor">
        <path d="M8 2 L9.5 6.5 L14 6.5 L10.5 9.5 L12 14 L8 11 L4 14 L5.5 9.5 L2 6.5 L6.5 6.5Z"/>
      </svg>
    ),
  };

  const el = paths[role] ?? paths["MIDDLE"];
  return <span className="shrink-0 text-text-muted opacity-60">{el}</span>;
}
