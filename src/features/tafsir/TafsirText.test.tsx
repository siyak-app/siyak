import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { TafsirText } from "./TafsirText";

function html(text: string) {
  const { container } = render(<TafsirText html={text} />);
  return container;
}

describe("TafsirText", () => {
  it("keeps the allowed subset", () => {
    const root = html(
      '<b>a</b> <i>b</i><br><span class="ayah">﴿c﴾</span> <span class="ref">d</span>' +
        '<span class="poem"><span class="hemistich">e</span></span>',
    );
    expect(root.querySelector("b")).toHaveTextContent("a");
    expect(root.querySelector("i")).toHaveTextContent("b");
    expect(root.querySelector("br")).toBeInTheDocument();
    expect(root.querySelector("span.text-primary")).toHaveTextContent("﴿c﴾");
    expect(root.querySelector("span.block span.block")).toHaveTextContent("e");
  });

  it("drops scripts, handlers and unknown markup but keeps the text", () => {
    const root = html(
      '<script>alert(1)</script><style>p{}</style><img src="x" onerror="alert(2)">' +
        '<a href="https://example.org" onclick="alert(3)">link</a>' +
        '<span class="evil" style="color:red" onmouseover="alert(4)">text</span>' +
        '<b onclick="alert(5)" id="x">bold</b><iframe src="https://example.org"></iframe>',
    );
    expect(root.textContent).toBe("linktextbold");
    expect(root.querySelector("script, style, img, a, iframe")).toBeNull();
    for (const element of root.querySelectorAll("*")) {
      expect(element.getAttributeNames().filter((name) => name !== "class")).toEqual([]);
    }
  });

  it("treats plain text with angle brackets safely", () => {
    expect(html("1 &lt; 2").textContent).toBe("1 < 2");
  });
});
