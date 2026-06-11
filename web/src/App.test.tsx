import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("starts on status page and navigates between sections", async () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "服务状态" })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "知识库" }));
    expect(screen.getByRole("heading", { name: "知识库" })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "维护扫描" }));
    expect(screen.getByRole("heading", { name: "维护扫描" })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "设置" }));
    expect(screen.getByRole("heading", { name: "设置" })).toBeInTheDocument();
  });
});
