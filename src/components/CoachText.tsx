export function CoachText({ text, muted }: { text: string; muted?: boolean }) {
  const textColor = muted ? "text-text-secondary" : "text-text-primary";

  return (
    <div className={`text-sm ${textColor} whitespace-pre-wrap leading-relaxed`}>
      {text.split("\n").map((line, i) => {
        const trimmed = line.trim();

        if (trimmed.startsWith("- ") || trimmed.startsWith("* ") || /^\d+[\.\)]\s/.test(trimmed)) {
          const bulletContent = trimmed.startsWith("- ") || trimmed.startsWith("* ")
            ? trimmed.slice(2)
            : trimmed.replace(/^\d+[\.\)]\s/, "");

          const parts = bulletContent.split(/(\*\*[^*]+\*\*)/g);

          return (
            <div key={i} className="flex gap-2 py-0.5">
              <span className="text-accent shrink-0">&#x2022;</span>
              <span>
                {parts.map((part, j) =>
                  part.startsWith("**") && part.endsWith("**") ? (
                    <strong key={j} className="text-accent font-semibold">
                      {part.slice(2, -2)}
                    </strong>
                  ) : (
                    <span key={j}>{part}</span>
                  )
                )}
              </span>
            </div>
          );
        }

        if (trimmed === "") return <div key={i} className="h-1" />;

        const parts = trimmed.split(/(\*\*[^*]+\*\*)/g);
        return (
          <p key={i}>
            {parts.map((part, j) =>
              part.startsWith("**") && part.endsWith("**") ? (
                <strong key={j} className="text-accent font-semibold">
                  {part.slice(2, -2)}
                </strong>
              ) : (
                <span key={j}>{part}</span>
              )
            )}
          </p>
        );
      })}
    </div>
  );
}
