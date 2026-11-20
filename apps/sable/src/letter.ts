export function tabLetter(title: string, url: string): string {
  const source = title.trim() || hostLabel(url);
  const match = source.match(/\p{L}|\p{N}/u);
  return (match?.[0] ?? "?").toLocaleUpperCase();
}

export function hostLabel(url: string): string {
  if (!url || url.startsWith("sable:")) return "";
  try {
    return new URL(url).host.replace(/^www\./, "");
  } catch {
    return "";
  }
}

export function omniboxValue(url: string): string {
  if (!url || url.startsWith("sable:")) return "";
  return url;
}
