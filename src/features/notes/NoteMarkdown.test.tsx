import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { NoteMarkdown } from "./NoteMarkdown";

describe("NoteMarkdown", () => {
  it("renders Markdown", () => {
    render(<NoteMarkdown>{"**kalın** metin"}</NoteMarkdown>);
    expect(screen.getByText("kalın").tagName).toBe("STRONG");
  });

  it("does not render HTML", () => {
    const { container } = render(
      <NoteMarkdown>{'<script>alert(1)</script><b onclick="x()">html</b>'}</NoteMarkdown>,
    );
    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("b")).toBeNull();
  });

  it("loads no images", () => {
    const { container } = render(
      <NoteMarkdown>{"![dış](https://example.com/a.png)"}</NoteMarkdown>,
    );
    expect(container.querySelector("img")).toBeNull();
  });

  it("keeps a link as text that cannot navigate", () => {
    const { container } = render(<NoteMarkdown>{"[kaynak](https://example.com)"}</NoteMarkdown>);
    expect(container.querySelector("a")).toBeNull();
    expect(screen.getByText("kaynak")).toHaveAttribute("title", "https://example.com");
  });
});
