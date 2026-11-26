import { expect, it, vi } from "vitest";
import { singleFlight } from "./bridge";
it("coalesces overlapping health refreshes and permits retry after failure", async () => {
  let reject!: (error: Error) => void;
  const action = vi.fn(
    () =>
      new Promise<void>((_resolve, fail) => {
        reject = fail;
      }),
  );
  const refresh = singleFlight(action);
  const first = refresh();
  const second = refresh();
  expect(first).toBe(second);
  expect(action).toHaveBeenCalledTimes(1);
  reject(new Error("Service unavailable"));
  await expect(first).rejects.toThrow("Service unavailable");
  const third = refresh();
  expect(action).toHaveBeenCalledTimes(2);
  reject(new Error("Still unavailable"));
  await expect(third).rejects.toThrow();
});
