import { describe, expect, it } from "vitest";
import { defaultLayout, normalizeLayout } from "./layout";
import { readJsonStorage } from "./storage";

describe("storage helpers", () => {
  it("falls back when stored JSON is corrupted", () => {
    localStorage.setItem("fuse.test.bad-layout", "{");

    const parsed = readJsonStorage("fuse.test.bad-layout", null);
    const layout = normalizeLayout(parsed);

    expect(layout).toEqual(defaultLayout);
  });
});
