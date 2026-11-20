export function Icon({ name }: { name: "back" | "forward" | "reload" | "plus" | "pin" | "close" | "split" }) {
  const common = {
    viewBox: "0 0 16 16",
    width: 16,
    height: 16,
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.5,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true,
  };
  if (name === "back") {
    return (
      <svg {...common}>
        <path d="M10 3.5 5.5 8 10 12.5" />
      </svg>
    );
  }
  if (name === "forward") {
    return (
      <svg {...common}>
        <path d="M6 3.5 10.5 8 6 12.5" />
      </svg>
    );
  }
  if (name === "reload") {
    return (
      <svg {...common}>
        <path d="M13 8a5 5 0 1 1-1.4-3.4" />
        <path d="M13 3.2V6.2H10" />
      </svg>
    );
  }
  if (name === "plus") {
    return (
      <svg {...common}>
        <path d="M8 3.5v9M3.5 8h9" />
      </svg>
    );
  }
  if (name === "pin") {
    return (
      <svg {...common}>
        <path d="M6 3.5h4l-.6 3.2 1.8 1.8H4.8l1.8-1.8zM8 8.5V13" />
      </svg>
    );
  }
  if (name === "split") {
    return (
      <svg {...common}>
        <rect x="2.5" y="3" width="4.5" height="10" rx="1" />
        <rect x="9" y="3" width="4.5" height="10" rx="1" />
      </svg>
    );
  }
  return (
    <svg {...common}>
      <path d="M4 4l8 8M12 4l-8 8" />
    </svg>
  );
}
