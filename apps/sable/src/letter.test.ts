import { expect, test } from "vitest";
import { hostLabel, omniboxValue, tabLetter } from "./letter";

test("pinned tabs use a letter from the title or host", () => {
  expect(tabLetter("Example Domain", "https://example.com")).toBe("E");
  expect(tabLetter("", "https://www.rust-lang.org/")).toBe("R");
  expect(tabLetter("New tab", "sable:newtab")).toBe("N");
});

test("the omnibox hides the internal new-tab address", () => {
  expect(omniboxValue("sable:newtab")).toBe("");
  expect(omniboxValue("https://example.com")).toBe("https://example.com");
  expect(hostLabel("https://www.example.com/docs")).toBe("example.com");
});
