import { App } from "./index";
import { type ZodSchema, z } from "zod";

/**
 * Passed to an RPC method handler when it is called to provide context and
 * utilities to the handler.
 */
export class RpcCtx<TParams> {
  constructor(public app: App, public name: string, public params: TParams) {}
}

type RpcHandler<TParams> = (ctx: RpcCtx<TParams>) => unknown;

export class RpcBuilder {
  constructor(public app: App, public name: string) {}

  /**
   * Allows the RPC method to be called with the specified input schema.
   *
   * If the RPC methods is created without specifiying an input schema, it
   * cannot be called with any parameters.
   *
   * @param schema Input schema for the RPC method
   */
  public input<TSchema extends ZodSchema>(
    schema: TSchema
  ): RpcBuilderWithInput<TSchema> {
    return new RpcBuilderWithInput<TSchema>(this, schema);
  }

  /**
   * Sets the handler function for the RPC method that can take any input.
   *
   * For strict typing of the input parameters, use the `input` method to
   * specify a schema, and then use the `handler` method on the returned
   * `RpcBuilderWithInput` instance to set the handler function.
   *
   * @param f The handler function for the RPC method that takes untyped input.
   */
  public handler<F extends RpcHandler<any>>(f: F) {
    const builder = new RpcBuilderWithInput(this, z.any());
    builder.handler(f);
  }
}

export class RpcBuilderWithInput<TSchema extends ZodSchema> {
  private _handler: RpcHandler<z.infer<TSchema>> | null = null;

  constructor(private _rpcBuilder: RpcBuilder, public schema: TSchema) {}

  /**
   * Finalizes the RPC method by registering it with the application.
   *
   * @param handler The handler function for the RPC method that takes typed
   * input.
   */
  public handler<F extends RpcHandler<z.infer<TSchema>>>(handler: F) {
    this._handler = handler;
    this._rpcBuilder.app._rpcMethods[this._rpcBuilder.name] = this;
  }

  public call(params: unknown) {
    if (!this._handler) {
      throw new Error(`RPC method ${this._rpcBuilder.name} has no handler`);
    }

    const parsed = this.schema.safeParse(params);
    if (!parsed.success) {
      throw new Error(
        `RPC method ${this._rpcBuilder.name} called with invalid parameters: ${parsed.error.message}`
      );
    }

    const ctx = new RpcCtx(
      this._rpcBuilder.app,
      this._rpcBuilder.name,
      parsed.data
    );

    return this._handler(ctx);
  }
}
