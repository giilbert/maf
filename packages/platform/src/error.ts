import { z } from "zod";

const errorBodySchema = z.object({
  type: z.literal("error"),
  data: z.object({
    message: z.string(),
  }),
});

export class PlatformApiError extends Error {
  cause: unknown;

  constructor(message: string, cause?: unknown) {
    super(message);

    this.name = "PlatformApiError";

    const result = errorBodySchema.safeParse(cause);
    if (result.success) {
      this.cause = result.data.data.message;
    } else {
      this.cause = cause;
    }
  }
}
